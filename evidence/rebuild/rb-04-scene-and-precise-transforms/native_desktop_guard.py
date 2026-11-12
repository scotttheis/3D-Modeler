"""Read-only input-desktop guard for the Windows native evidence harnesses."""
import ctypes
from ctypes import wintypes

_user = ctypes.WinDLL('user32', use_last_error=True)
_user.OpenInputDesktop.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
_user.OpenInputDesktop.restype = wintypes.HANDLE
_user.GetUserObjectInformationW.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
_user.CloseDesktop.argtypes = [wintypes.HANDLE]

def require_default_desktop():
    handle = _user.OpenInputDesktop(0, False, 0x0001)
    if not handle:
        raise RuntimeError(f'Input desktop unavailable (Win32 {ctypes.get_last_error()}); no input or capture sent')
    try:
        name = ctypes.create_unicode_buffer(256)
        needed = wintypes.DWORD()
        if not _user.GetUserObjectInformationW(handle, 2, name, ctypes.sizeof(name), ctypes.byref(needed)):
            raise RuntimeError('Cannot verify input desktop; no input or capture sent')
        if name.value.casefold() != 'default':
            raise RuntimeError('Input desktop is locked/secure; no input or capture sent')
        return name.value
    finally:
        _user.CloseDesktop(handle)
