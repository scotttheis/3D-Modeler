use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The owner of an accounted allocation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MemoryDomain {
    /// System RAM, including CPU-authoritative scene data and unified memory.
    System,
    /// Memory visible through a graphics adapter.
    Adapter(usize),
}

/// The subsystem responsible for an allocation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AllocationKind {
    /// Exact scene or mesh data used for editing and export.
    Authoritative,
    /// Undo/redo snapshots.
    UndoHistory,
    /// Data waiting for a device transfer.
    Staging,
    /// Render-target textures and depth buffers.
    RenderTarget,
    /// Revision-keyed derived data that can be evicted.
    Cache,
    /// Exact picking acceleration data.
    Picking,
    /// Work owned by a background operation.
    Background,
}

/// A resource budget for one adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterBudget {
    /// The live budget available to this adapter in bytes.
    pub budget_bytes: u64,
    /// Whether adapter allocations draw from the same physical pool as RAM.
    pub unified_memory: bool,
}

/// Stable identifier returned for a successful reservation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct AllocationId(u64);

impl AllocationId {
    /// Returns the opaque numeric value for diagnostics and tests.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
struct AdapterLedger {
    budget_bytes: u64,
    used_bytes: u64,
    unified_memory: bool,
}

#[derive(Clone, Copy, Debug)]
struct AllocationRecord {
    kind: AllocationKind,
    domain: MemoryDomain,
    bytes: u64,
    system_charge_bytes: u64,
    adapter_charge_bytes: u64,
}

/// Errors returned when a reservation cannot be represented safely.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LedgerError {
    /// An adapter index did not exist in this ledger.
    #[error("adapter index {0} is not present in the resource ledger")]
    UnknownAdapter(usize),
    /// The requested reservation would exceed a live budget.
    #[error("{domain:?} allocation of {requested} bytes exceeds the available {available} bytes")]
    BudgetExceeded {
        /// The domain that was over budget.
        domain: MemoryDomain,
        /// The requested allocation size.
        requested: u64,
        /// The remaining budget at the time of the request.
        available: u64,
    },
    /// The caller attempted to release an unknown reservation.
    #[error("allocation {0:?} is not present in the resource ledger")]
    UnknownAllocation(AllocationId),
    /// An accounting counter would overflow.
    #[error("resource accounting overflow")]
    AccountingOverflow,
}

/// A bounded, central accounting ledger for CPU, GPU, shared, and in-flight data.
#[derive(Clone, Debug)]
pub struct ResourceLedger {
    system_budget_bytes: u64,
    system_used_bytes: u64,
    adapters: Vec<AdapterLedger>,
    next_allocation_id: u64,
    allocations: BTreeMap<AllocationId, AllocationRecord>,
}

impl ResourceLedger {
    /// Creates a ledger from the currently observed system and adapter budgets.
    #[must_use]
    pub fn new(system_budget_bytes: u64, adapter_budgets: Vec<AdapterBudget>) -> Self {
        let adapters = adapter_budgets
            .into_iter()
            .map(|budget| AdapterLedger {
                budget_bytes: budget.budget_bytes,
                used_bytes: 0,
                unified_memory: budget.unified_memory,
            })
            .collect();
        Self {
            system_budget_bytes,
            system_used_bytes: 0,
            adapters,
            next_allocation_id: 1,
            allocations: BTreeMap::new(),
        }
    }

    /// Returns the currently available bytes in a memory domain.
    pub fn available_bytes(&self, domain: MemoryDomain) -> Result<u64, LedgerError> {
        match domain {
            MemoryDomain::System => Ok(self
                .system_budget_bytes
                .saturating_sub(self.system_used_bytes)),
            MemoryDomain::Adapter(index) => self
                .adapters
                .get(index)
                .map(|adapter| adapter.budget_bytes.saturating_sub(adapter.used_bytes))
                .ok_or(LedgerError::UnknownAdapter(index)),
        }
    }

    /// Returns bytes currently charged to one memory domain.
    pub fn used_bytes(&self, domain: MemoryDomain) -> Result<u64, LedgerError> {
        match domain {
            MemoryDomain::System => Ok(self.system_used_bytes),
            MemoryDomain::Adapter(index) => self
                .adapters
                .get(index)
                .map(|adapter| adapter.used_bytes)
                .ok_or(LedgerError::UnknownAdapter(index)),
        }
    }

    /// Returns the live budget for one memory domain.
    pub fn budget_bytes(&self, domain: MemoryDomain) -> Result<u64, LedgerError> {
        match domain {
            MemoryDomain::System => Ok(self.system_budget_bytes),
            MemoryDomain::Adapter(index) => self
                .adapters
                .get(index)
                .map(|adapter| adapter.budget_bytes)
                .ok_or(LedgerError::UnknownAdapter(index)),
        }
    }

    /// Returns the fraction of a domain's live budget currently reserved.
    pub fn pressure_ratio(&self, domain: MemoryDomain) -> Result<f64, LedgerError> {
        let budget = self.budget_bytes(domain)?;
        let used = self.used_bytes(domain)?;
        if budget == 0 {
            return Ok(if used == 0 { 0.0 } else { 1.0 });
        }
        Ok((used as f64 / budget as f64).clamp(0.0, 1.0))
    }

    /// Reserves bytes and returns an ID that must be released when the data dies.
    pub fn reserve(
        &mut self,
        kind: AllocationKind,
        domain: MemoryDomain,
        bytes: u64,
    ) -> Result<AllocationId, LedgerError> {
        let (system_charge_bytes, adapter_charge_bytes) = match domain {
            MemoryDomain::System => (bytes, 0),
            MemoryDomain::Adapter(index) => {
                let adapter = self
                    .adapters
                    .get(index)
                    .ok_or(LedgerError::UnknownAdapter(index))?;
                (if adapter.unified_memory { bytes } else { 0 }, bytes)
            }
        };

        let system_available = self
            .system_budget_bytes
            .saturating_sub(self.system_used_bytes);
        if system_charge_bytes > system_available {
            return Err(LedgerError::BudgetExceeded {
                domain: MemoryDomain::System,
                requested: system_charge_bytes,
                available: system_available,
            });
        }

        if let MemoryDomain::Adapter(index) = domain {
            let adapter = self
                .adapters
                .get(index)
                .ok_or(LedgerError::UnknownAdapter(index))?;
            let available = adapter.budget_bytes.saturating_sub(adapter.used_bytes);
            if adapter_charge_bytes > available {
                return Err(LedgerError::BudgetExceeded {
                    domain,
                    requested: adapter_charge_bytes,
                    available,
                });
            }
        }

        let allocation_id = AllocationId(self.next_allocation_id);
        self.next_allocation_id = self
            .next_allocation_id
            .checked_add(1)
            .ok_or(LedgerError::AccountingOverflow)?;
        self.system_used_bytes = self
            .system_used_bytes
            .checked_add(system_charge_bytes)
            .ok_or(LedgerError::AccountingOverflow)?;
        if let MemoryDomain::Adapter(index) = domain {
            let adapter = self
                .adapters
                .get_mut(index)
                .ok_or(LedgerError::UnknownAdapter(index))?;
            adapter.used_bytes = adapter
                .used_bytes
                .checked_add(adapter_charge_bytes)
                .ok_or(LedgerError::AccountingOverflow)?;
        }
        self.allocations.insert(
            allocation_id,
            AllocationRecord {
                kind,
                domain,
                bytes,
                system_charge_bytes,
                adapter_charge_bytes,
            },
        );
        Ok(allocation_id)
    }

    /// Releases a previous reservation exactly once.
    pub fn release(&mut self, allocation_id: AllocationId) -> Result<(), LedgerError> {
        let record = self
            .allocations
            .remove(&allocation_id)
            .ok_or(LedgerError::UnknownAllocation(allocation_id))?;
        self.system_used_bytes = self
            .system_used_bytes
            .checked_sub(record.system_charge_bytes)
            .ok_or(LedgerError::AccountingOverflow)?;
        if let MemoryDomain::Adapter(index) = record.domain {
            let adapter = self
                .adapters
                .get_mut(index)
                .ok_or(LedgerError::UnknownAdapter(index))?;
            adapter.used_bytes = adapter
                .used_bytes
                .checked_sub(record.adapter_charge_bytes)
                .ok_or(LedgerError::AccountingOverflow)?;
        }
        Ok(())
    }

    /// Returns a compact snapshot suitable for diagnostics and routing input.
    #[must_use]
    pub fn snapshot(&self) -> ResourceLedgerSnapshot {
        ResourceLedgerSnapshot {
            system_budget_bytes: self.system_budget_bytes,
            system_used_bytes: self.system_used_bytes,
            system_available_bytes: self
                .system_budget_bytes
                .saturating_sub(self.system_used_bytes),
            adapters: self
                .adapters
                .iter()
                .map(|adapter| {
                    (
                        adapter.budget_bytes,
                        adapter.used_bytes,
                        adapter.unified_memory,
                    )
                })
                .collect(),
            adapter_available_bytes: self
                .adapters
                .iter()
                .map(|adapter| adapter.budget_bytes.saturating_sub(adapter.used_bytes))
                .collect(),
            allocation_count: self.allocations.len(),
            allocations_by_kind: self.allocation_totals(),
        }
    }

    fn allocation_totals(&self) -> BTreeMap<AllocationKind, u64> {
        let mut totals = BTreeMap::new();
        for record in self.allocations.values() {
            let entry = totals.entry(record.kind).or_insert(0_u64);
            *entry = (*entry).saturating_add(record.bytes);
        }
        totals
    }
}

/// A serializable ledger snapshot used by telemetry and the R0b report.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceLedgerSnapshot {
    /// System-memory budget in bytes.
    pub system_budget_bytes: u64,
    /// System-memory bytes currently accounted.
    pub system_used_bytes: u64,
    /// System-memory bytes still available to the ledger.
    pub system_available_bytes: u64,
    /// `(budget, used, unified)` for each adapter.
    pub adapters: Vec<(u64, u64, bool)>,
    /// Adapter-memory bytes still available for each adapter.
    pub adapter_available_bytes: Vec<u64>,
    /// Number of live reservations.
    pub allocation_count: usize,
    /// Bytes grouped by allocation owner.
    pub allocations_by_kind: BTreeMap<AllocationKind, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_adapter_reservation_counts_against_one_system_pool() {
        let mut ledger = ResourceLedger::new(
            100,
            vec![AdapterBudget {
                budget_bytes: 80,
                unified_memory: true,
            }],
        );
        let allocation = ledger
            .reserve(AllocationKind::Authoritative, MemoryDomain::Adapter(0), 60)
            .map_err(|error| error.to_string());
        assert!(allocation.is_ok());
        assert_eq!(ledger.available_bytes(MemoryDomain::System), Ok(40));
        assert_eq!(ledger.available_bytes(MemoryDomain::Adapter(0)), Ok(20));
        let snapshot = ledger.snapshot();
        assert_eq!(snapshot.system_available_bytes, 40);
        assert_eq!(snapshot.adapter_available_bytes, vec![20]);
        assert_eq!(ledger.used_bytes(MemoryDomain::Adapter(0)), Ok(60));
        assert_eq!(ledger.budget_bytes(MemoryDomain::Adapter(0)), Ok(80));
        assert_eq!(ledger.pressure_ratio(MemoryDomain::Adapter(0)), Ok(0.75));
    }

    #[test]
    fn pressure_is_rejected_before_accounting_changes() {
        let mut ledger = ResourceLedger::new(
            100,
            vec![AdapterBudget {
                budget_bytes: 50,
                unified_memory: false,
            }],
        );
        let error = ledger
            .reserve(AllocationKind::RenderTarget, MemoryDomain::Adapter(0), 51)
            .err();
        assert_eq!(
            error,
            Some(LedgerError::BudgetExceeded {
                domain: MemoryDomain::Adapter(0),
                requested: 51,
                available: 50,
            })
        );
        assert_eq!(ledger.snapshot().allocation_count, 0);
    }

    #[test]
    fn releasing_a_reservation_restores_capacity() {
        let mut ledger = ResourceLedger::new(100, Vec::new());
        let allocation = ledger
            .reserve(AllocationKind::Cache, MemoryDomain::System, 40)
            .map_err(|error| error.to_string());
        assert!(allocation.is_ok());
        let allocation_id = match allocation {
            Ok(id) => id,
            Err(_) => return,
        };
        assert_eq!(ledger.available_bytes(MemoryDomain::System), Ok(60));
        assert!(ledger.release(allocation_id).is_ok());
        assert_eq!(ledger.available_bytes(MemoryDomain::System), Ok(100));
        assert_eq!(
            ledger.release(allocation_id),
            Err(LedgerError::UnknownAllocation(allocation_id))
        );
    }
}
