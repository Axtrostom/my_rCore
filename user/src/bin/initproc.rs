#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

#[unsafe(no_mangle)]
fn main() -> i32 {
    ///进来直接fork ，然后子进程和父进程都会执行结下来的内容
    ///不同的是，子进程fork后返回值为 0 ，父进程 fork 后返回值为子进程 pid
    if fork() == 0 {//子进程
        exec("user_shell\0");//执行 shell
    } else {//父进程
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);//等待任意子进程结束
            if pid == -1 {//没有子进程了
                yield_();
                continue;
            }
            println!(//释放一个僵尸进程
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code,
            );
        }
    }
    0
}
