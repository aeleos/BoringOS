.intel_syntax noprefix

.global check_cpuid
.global check_long_mode

.extern early_error

.section .init
.code32
check_cpuid:
    # Check if CPUID is supported by attempting to flip the ID bit (bit 21)
    # in the FLAGS register. If we can flip it, CPUID is available.

    # Copy FLAGS in to EAX via stack
    pushfd
    pop eax

    # Copy to ECX as well for comparing later on
    mov ecx, eax

    # Flip the ID bit
    xor eax, (1 << 21)

    # Copy EAX to FLAGS via the stack
    push eax
    popfd

    # Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax

    # Restore FLAGS from the old version stored in ECX (i.e. flipping the
    # ID bit back if it was ever flipped).
    push ecx
    popfd

    # Compare EAX and ECX. If they are equal then that means the bit
    # wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je no_cpuid
    ret
no_cpuid:
    lea si, no_cpuid_str
    jmp error

check_long_mode:
    # test if extended processor info in available
    mov eax, 0x80000000    # implicit argument for cpuid
    cpuid                  # get highest supported argument
    cmp eax, 0x80000001    # it needs to be at least 0x80000001
    jb no_long_mode        # if it's less, the CPU is too old for long mode

    # use extended info to test if long mode is available
    mov eax, 0x80000001    # argument for extended processor info
    cpuid                  # returns various feature bits in ecx and edx
    test edx, (1 << 29)    # test if the LM-bit is set in the D-register
    jz no_long_mode        # If it's not set, there is no long mode
    ret
no_long_mode:
    lea si, no_long_mode_error
    jmp error

no_long_mode_error:
    .asciz "Your CPU does not support 64 Bit mode. That means it's not possible to use this OS on your computer. Aborting..."

# print a string and a newline
# IN
#   esi: points at zero-terminated String
# CLOBBER
#   ax
println:
    call print
    mov al, 13 # \r
    call print_char
    mov al, 10 # \n
    jmp print_char

# print a string
# IN
#   esi: points at zero-terminated String
# CLOBBER
#   ax
print:
    cld
print_loop:
    lodsb al, BYTE PTR [esi]
    test al, al
    jz print_done
    call print_char
    jmp print_loop
print_done:
    ret

# print a character
# IN
#   al: character to print
# CLOBBER
#   ah
print_char:
    mov ah, 0x0e
    int 0x10
    ret

# print a number in hex
# IN
#   bx: the number
# CLOBBER
#   al, cx
print_hex:
    mov cx, 4
.lp:
    mov al, bh
    shr al, 4

    cmp al, 0xA
    jb .below_0xA

    add al, 'A' - 0xA - '0'
.below_0xA:
    add al, '0'

    call print_char

    shl bx, 4
    loop .lp

    ret

error:
    call println
    jmp spin

no_int13h_extensions:
    lea si, no_int13h_extensions_str
    jmp error

second_stage_load_failed:
    lea si, second_stage_load_failed_str
    jmp error

kernel_load_failed:
    lea si, kernel_load_failed_str
    jmp error

spin:
        jmp spin


no_long_mode_str: .asciz "No long mode support"
boot_start_str: .asciz "Booting (first stage)..."
second_stage_start_str: .asciz "Booting (second stage)..."
error_str: .asciz "Error: "
no_cpuid_str: .asciz "No CPUID support"
no_int13h_extensions_str: .asciz "No support for int13h extensions"
second_stage_load_failed_str: .asciz "Failed to load second stage of bootloader"
loading_kernel_block_str: .asciz "loading kernel block..."
kernel_load_failed_str: .asciz "Failed to load kernel"
