fn main() {
    #[cfg(windows)]
    windows::build! {
        Windows::Win32::Foundation::CloseHandle,
        Windows::Win32::System::Diagnostics::Debug::FlushInstructionCache,
        Windows::Win32::System::Memory::*,
        Windows::Win32::System::SystemInformation::GetSystemInfo,
        Windows::Win32::System::Threading::GetCurrentProcess,
    }
}
