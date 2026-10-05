#include <stdint.h>
#include <stddef.h>

uint64_t syscall0(uint64_t syscall_number);
uint64_t syscall1(uint64_t syscall_number, uint64_t args0);
uint64_t syscall2(uint64_t syscall_number, uint64_t args0,
						uint64_t args1);
uint64_t syscall3(uint64_t syscall_number, uint64_t args0,
						uint64_t args1, uint64_t args2);
uint64_t syscall4(uint64_t syscall_number, uint64_t args0,
						uint64_t args1, uint64_t args2, uint64_t args3);
uint64_t syscall5(uint64_t syscall_number, uint64_t args0,
						uint64_t args1, uint64_t args2, uint64_t args3,
						uint64_t args4);
uint64_t syscall6(uint64_t syscall_number, uint64_t args0,
						uint64_t args1, uint64_t args2, uint64_t args3,
						uint64_t args4, uint64_t args5);


void syslog(const char *string) {
    syscall1(0, (uintptr_t)string);
}

void _start(void) {
    syslog("Hello from userspace!\r\n");
    for (;;);
}
