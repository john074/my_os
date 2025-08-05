# my_os (ENG)

**my_os** is a hobby operating system written primarily in Rust.

## Project Purpose

The main purpose of this project is to gain a deeper understanding of how operating systems work at a low level. It is based on Philipp Oppermann’s excellent tutorial ["Writing an OS in Rust"](https://os.phil-opp.com/) (both [first edition](https://os.phil-opp.com/edition-1/) and [second edition](https://os.phil-opp.com/)) and extends it with additional features such as:

- A basic file system
- Support for running custom user programs (including a shell)
- Planned support for:
  - Preemptive multitasking with native threads (currently green threads are used)
  - Networking
  - Graphics mode support

## Dependencies

### Kernel

```toml
[dependencies]
volatile = "0.2.6"
spin = "0.10.0"
x86_64 = "0.14.2"
pic8259 = "0.10.1"
pc-keyboard = "0.7.0"
rlibc = "1.0"
multiboot2 = "0.22.0"
bitflags = "0.9.1"
x86 = "0.52.0"
xmas-elf = "0.9.1"

[dependencies.lazy_static]
version = "1.0"
features = ["spin_no_std"] 

[dependencies.crossbeam-queue]
version = "0.3.11"
default-features = false
features = ["alloc"]

[dependencies.futures-util]
version = "0.3.4"
default-features = false
features = ["alloc"]

[dependencies.conquer-once]
version = "0.2.0"
default-features = false

[patch.crates-io]
compiler_builtins = { 
    git = "https://github.com/rust-lang/compiler-builtins", 
    branch = "master", 
    features = ["mem"], 
    default-features = false 
}

[build-dependencies]
compiler_builtins = {  
    git = "https://github.com/rust-lang/compiler-builtins", 
    branch = "master", 
    features = ["mem"], 
    default-features = false 
}
```

### User-space Standard Library

```toml
[dependencies]
volatile = "0.2.6"
spin = "0.10.0"
x86_64 = "0.14.2"

[dependencies.lazy_static]
version = "1.0"
features = ["spin_no_std"] 

[dependencies.crossbeam-queue]
version = "0.3.11"
default-features = false
features = ["alloc"]

[dependencies.futures-util]
version = "0.3.4"
default-features = false
features = ["alloc"]
```

## Building

**Linux only.**  
You will need: `make`, `cargo`, Rust **nightly**, `nasm`, `ld`, and `qemu`.

After cloning the repo, run:

```sh
make all
```

This will automatically compile the kernel and boot it in QEMU.

## Writing and Running Custom Programs

To create your own user program:

1. Copy an existing directory `usr/programs/test_program`.
2. Change the linker address in the `Makefile` (e.g., from `0x444444840000` to something upper like `0x444444940000` (1mb step is recommended)).
3. Edit `src/main.rs` with your code.

### Minimal entrypoint

If you just want to run a single function:

```rust
#[no_mangle]
pub extern "C" fn _start() {
    // Your code here
    exit()
}
```

### Asynchronous task entrypoint

If you want to run your code as an async task:

```rust
#[no_mangle]
pub extern "C" fn _start() {
    let mut task = multitasking::Task::new(user_main());
    syscall::spawn_task((&mut task as *mut multitasking::Task) as u64);
}

async fn user_main() {
    // Your async code here
    // Use multitasking::cooperate().await to yield to other tasks
    exit()
}
```

After that, just run `make all` again from the root directory, and your program will be compiled and loaded with the OS.

---

# my_os (RU)

**my_os** — любительская-операционная система на Rust.

## Цель проекта

Основная цель проекта — лучше понять, как работают операционные системы "под капотом". Проект основан на руководстве Филиппа Оппермана ["Writing an OS in Rust"](https://os.phil-opp.com/) (оба издания: [первое](https://os.phil-opp.com/edition-1/) и [второе](https://os.phil-opp.com/)) и расширен следующими возможностями:

- Простейшая файловая система
- Запуск пользовательских программ (включая shell)
- В планах:
  - Вытесняющая многозадачность с использованием настоящих потоков (пока используются зелёные потоки)
  - Минимальная поддержка работы с сетью 
  - Графический режим

## Зависимости



Смотри секцию `[dependencies]` выше


## Сборка

**Только для Linux.**  
Требуется наличие: `make`, `cargo`, Rust **nightly**, `nasm`, `ld`, и `qemu`.

После клонирования репозитория выполните:

```sh
make all
```

Это соберёт ядро и запустит его в QEMU.

## Написание и запуск пользовательских программ

Чтобы создать свою программу:

1. Скопируйте папку `usr/programs/test_program`.
2. Измените адрес линковки в `Makefile` (с `0x444444840000` на адрес выше, например, `0x444444940000` (рекомендуется шаг в 1 мб)).
3. В `src/main.rs` замените код на ваш.

### Минимальная точка входа

```rust
#[no_mangle]
pub extern "C" fn _start() {
    // Ваш код
    exit()
}
```

### Асинхронная задача

```rust
#[no_mangle]
pub extern "C" fn _start() {
    let mut task = multitasking::Task::new(user_main());
    syscall::spawn_task((&mut task as *mut multitasking::Task) as u64);
}

async fn user_main() {
    // Ваш асинхронный код
    // Используйте multitasking::cooperate().await для передачи управления другим задачам
    exit()
}
```

Затем выполните `make all` из корня проекта, чтобы собрать и записисать вашу программу на образ диска вместе с ОС.
