#![no_std]

#[macro_use]
extern crate veos_std;
extern crate rlibc;

#[no_mangle]
pub fn main() {
    let pid = veos_std::process::get_pid();
    veos_std::process::exec("/bin/test");
    //loop {
        print!("{}", pid);
    //}
}