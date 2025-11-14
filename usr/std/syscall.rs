#[repr(u64)]
pub enum SyscallNumber {
    Write = 1,
    Exit = 2,
    Read = 3,
    RemoveChar = 4,
    SpawnTask = 5,
    Alloc = 6,
    Dealloc = 7,
    CheckFsEntryExists = 8,
    GenerateTaskId = 9,
    ListDir = 10,
    MakeDir = 11,
    MakeFile = 12,
    WriteFile = 13,
    ReadFile = 14,
    RemoveDir = 15,
    RemoveFile = 16,
    Run = 17,
    ClearScreen = 18,
}

pub fn syscall(n: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") n,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r8") arg4,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}

pub fn exit() {
    syscall(SyscallNumber::Exit as u64, 0, 0, 0, 0);
}

pub fn syscall_alloc(size: u64, align: u64) -> u64 {
    syscall(SyscallNumber::Alloc as u64, size, align, 0, 0)
}

pub fn syscall_dealloc(ptr: u64, size: u64, align: u64) {
    syscall(SyscallNumber::Dealloc as u64, ptr, size, align, 0);
}

pub fn spawn_task(task_addr: u64) {
    syscall(SyscallNumber::SpawnTask as u64, task_addr, 0, 0, 0);
}

pub fn read(buffer: u64) -> u64 {
    syscall(SyscallNumber::Read as u64, buffer, 0, 0, 0)
}

pub fn rm_char() -> u64 {
    syscall(SyscallNumber::RemoveChar as u64, 0, 0, 0, 0)
}

pub fn check_fs_entry_exists(path: &str) -> u64 {
	syscall(SyscallNumber::CheckFsEntryExists as u64, path.as_ptr() as u64, path.len() as u64, 0, 0)
}

pub fn ls(path: &str, buffer: u64) -> u64 {
	syscall(SyscallNumber::ListDir as u64, path.as_ptr() as u64, path.len() as u64, buffer, 0)
}

pub fn mkdir(path: &str) -> u64 {
	syscall(SyscallNumber::MakeDir as u64, path.as_ptr() as u64, path.len() as u64, 0, 0)
}

pub fn mkfile(path: &str) -> u64 {
	syscall(SyscallNumber::MakeFile as u64, path.as_ptr() as u64, path.len() as u64, 0, 0)
}

pub fn write_file(path: &str, content: &str) -> u64 {
	syscall(SyscallNumber::WriteFile as u64, path.as_ptr() as u64, path.len() as u64, content.as_ptr() as u64, content.len() as u64)
}

pub fn read_file(path: &str, buffer: u64, len: u64) -> u64 {
	syscall(SyscallNumber::ReadFile as u64, path.as_ptr() as u64, path.len() as u64, buffer, len)
}

pub fn rmdir(path: &str) -> u64 {
	syscall(SyscallNumber::RemoveDir as u64, path.as_ptr() as u64, path.len() as u64, 0, 0)
}

pub fn rmfile(path: &str) -> u64 {
	syscall(SyscallNumber::RemoveFile as u64, path.as_ptr() as u64, path.len() as u64, 0, 0)
}

pub fn run(path: &str) -> u64 {
	syscall(SyscallNumber::Run as u64, path.as_ptr() as u64, path.len() as u64, 0, 0)
}

pub fn get_task_id() -> u64 {
	syscall(SyscallNumber::GenerateTaskId as u64, 0, 0, 0, 0)
}

pub fn clear_screen() -> u64 {
    syscall(SyscallNumber::ClearScreen as u64, 0, 0, 0, 0)
}

