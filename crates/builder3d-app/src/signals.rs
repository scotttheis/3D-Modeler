//! Toolkit-neutral typed callback signals.
//!
//! The Python desktop uses Qt signals at the UI boundary. Rust adapters use
//! typed callbacks instead, keeping event routing testable without importing a
//! GUI runtime into the model or application services.

use std::collections::BTreeMap;

type Callback<T> = Box<dyn Fn(&T)>;

/// Stable handle returned when a callback is connected.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignalConnection(usize);

/// A synchronous, typed callback signal.
pub struct Signal<T> {
    next_id: usize,
    callbacks: BTreeMap<usize, Callback<T>>,
}

impl<T> Default for Signal<T> {
    fn default() -> Self {
        Self {
            next_id: 0,
            callbacks: BTreeMap::new(),
        }
    }
}

impl<T> Signal<T> {
    /// Registers a callback and returns its disconnect handle.
    pub fn connect<F>(&mut self, callback: F) -> SignalConnection
    where
        F: Fn(&T) + 'static,
    {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.callbacks.insert(id, Box::new(callback));
        SignalConnection(id)
    }

    /// Removes a callback, returning whether a callback was removed.
    pub fn disconnect(&mut self, connection: SignalConnection) -> bool {
        self.callbacks.remove(&connection.0).is_some()
    }

    /// Emits one value to a snapshot of the current callback set.
    pub fn emit(&self, value: &T) {
        for callback in self.callbacks.values() {
            callback(value);
        }
    }

    /// Returns the number of connected callbacks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    /// Returns whether no callbacks are connected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    #[test]
    fn typed_signal_emits_in_connection_order_and_disconnects_by_handle() {
        let mut signal = Signal::<u32>::default();
        let events = Rc::new(RefCell::new(Vec::new()));
        let first_events = Rc::clone(&events);
        let first = signal.connect(move |value| first_events.borrow_mut().push(*value));
        let second_events = Rc::clone(&events);
        let second = signal.connect(move |value| second_events.borrow_mut().push(*value + 1));
        assert_eq!(signal.len(), 2);
        signal.emit(&4);
        assert_eq!(*events.borrow(), vec![4, 5]);
        assert!(signal.disconnect(first));
        assert!(!signal.disconnect(first));
        signal.emit(&8);
        assert_eq!(*events.borrow(), vec![4, 5, 9]);
        assert!(signal.disconnect(second));
        assert!(signal.is_empty());
    }
}
