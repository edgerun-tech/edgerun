; EdgeRun memory operations — included by runtime.asm

; ==================================================================
; er_memset — fill memory with a byte value
; void* er_memset(void* dst, int value, size_t num)
; Returns: dst
; ==================================================================
er_fn er_memset
    mov     r8, rdi             ; r8 = dst (save before clobbering)
    mov     rcx, rdx            ; rcx = count
    movzx   eax, sil            ; eax = value byte (zero-extended)

    cld
    mov     r9, rcx
    and     r9, 7
    shr     rcx, 3
    rep     stosq               ; qword fill (8x fewer iterations)
    mov     rcx, r9
    rep     stosb               ; remaining bytes

    mov     rax, r8             ; return dst
    ret

; ==================================================================
; er_memcpy — copy memory buffer
; void* er_memcpy(void* dst, const void* src, size_t num)
; Returns: dst
; ==================================================================
er_fn er_memcpy
    mov     rax, rdi            ; retval = dst

    mov     rcx, rdx            ; rcx = num
    mov     rsi, rsi            ; rsi = src (already in rsi)
    mov     rdi, rax            ; rdi = dst
    cld
    mov     r8, rcx
    and     r8, 7
    shr     rcx, 3
    rep     movsq               ; qword copy (8x fewer iterations)
    mov     rcx, r8
    rep     movsb               ; remaining bytes

    ret

; ==================================================================
; er_memcmp — compare two memory buffers
; int er_memcmp(const void* ptr1, const void* ptr2, size_t num)
; Returns: 0 if equal, <0 if ptr1<ptr2, >0 if ptr1>ptr2
; ==================================================================
er_fn er_memcmp
    er_check_zero rdx, .equal

.loop:
    movzx   eax, byte [rdi]
    movzx   ecx, byte [rsi]
    cmp     eax, ecx
    jne     .diff
    inc     rdi
    inc     rsi
    dec     rdx
    jnz     .loop

.equal:
    xor     eax, eax
    ret

.diff:
    sub     eax, ecx
    ret

; ==================================================================
; er_memmove — copy memory (handles overlapping buffers)
; void* er_memmove(void* dst, const void* src, size_t num)
; Returns: dst
; ==================================================================
er_fn er_memmove
    mov     rax, rdi            ; retval = dst

    cmp     rdi, rsi
    jbe     .forward

    lea     rcx, [rsi + rdx]    ; rcx = src + num
    cmp     rdi, rcx
    jae     .forward

    lea     rsi, [rsi + rdx - 1] ; src end
    lea     rdi, [rdi + rdx - 1] ; dst end
    mov     rcx, rdx
    mov     r8, rcx
    and     r8, 7
    shr     rcx, 3
    std
    rep     movsq
    mov     rcx, r8
    rep     movsb
    cld
    ret

.forward:
    mov     rcx, rdx
    mov     r8, rcx
    and     r8, 7
    shr     rcx, 3
    cld
    rep     movsq
    mov     rcx, r8
    rep     movsb
    ret

; ==================================================================
; er_bss_zero — zero the BSS section
; void er_bss_zero(void* bss_start, void* bss_end)
; Called from _start before any C/asm code runs
; ==================================================================
er_fn er_bss_zero
    mov     rcx, rsi
    sub     rcx, rdi
    jle     .done

    xor     eax, eax
    cld
    mov     rdi, rdi
    shr     rcx, 3
    rep     stosq

.done:
    ret

; ==================================================================
; er_memchr — find first occurrence of byte in memory buffer
; void* er_memchr(const void* ptr, int value, size_t num)
; Returns: pointer to first occurrence, or NULL if not found
; ==================================================================
er_fn er_memchr
    er_check_zero rdx, .memchr_not_found
    mov     rcx, rdx
    mov     al, sil
    mov     rdi, rdi
    cld
    repne   scasb
    jne     .memchr_not_found
    lea     rax, [rdi - 1]
    er_ret
.memchr_not_found:
    xor     eax, eax
    er_ret

; ==================================================================
; er_memrchr — find last occurrence of byte in memory buffer (reverse)
; void* er_memrchr(const void* ptr, int value, size_t num)
; Returns: pointer to last occurrence, or NULL if not found
; ==================================================================
er_fn er_memrchr
    er_check_zero rdx, .memrchr_not_found
    mov     rcx, rdx
    lea     rdi, [rdi + rcx - 1]
    mov     al, sil
    std
    repne   scasb
    cld
    jne     .memrchr_not_found
    lea     rax, [rdi + 1]
    er_ret
.memrchr_not_found:
    cld
    xor     eax, eax
    er_ret

; ==================================================================
; er_memset32 — fill memory with 32-bit value (4-byte aligned)
; void* er_memset32(void* dst, uint32_t value, size_t count)
; count is number of 32-bit elements, NOT bytes
; Returns: dst
; ==================================================================
er_fn er_memset32
    mov     r8, rdi
    mov     rcx, rdx
    mov     eax, esi
    cld
    rep     stosd
    mov     rax, r8
    er_ret

; ==================================================================
; er_memset64 — fill memory with 64-bit value (8-byte aligned)
; void* er_memset64(void* dst, uint64_t value, size_t count)
; count is number of 64-bit elements, NOT bytes
; Returns: dst
; ==================================================================
er_fn er_memset64
    mov     r8, rdi
    mov     rcx, rdx
    mov     rax, rsi
    cld
    rep     stosq
    mov     rax, r8
    er_ret

; ==================================================================
; er_memccpy — copy memory until stop_char found or limit reached
; void* er_memccpy(void* dst, const void* src, int stop_char, size_t n)
; Returns: pointer to byte AFTER stop_char in dst, or NULL if not found
; ==================================================================
er_fn er_memccpy
    mov     r8, rdi
    mov     r9, rsi
    mov     al, dl
.memccpy_loop:
    er_check_zero rcx, .memccpy_null
    mov     dl, byte [r9]
    mov     byte [r8], dl
    inc     r8
    inc     r9
    dec     rcx
    cmp     dl, al
    jne     .memccpy_loop
    lea     rax, [r8]
    er_ret
.memccpy_null:
    xor     eax, eax
    er_ret

; ==================================================================
; er_memicmp — case-insensitive memory compare
; int er_memicmp(const void* ptr1, const void* ptr2, size_t n)
; Folds A-Z to lowercase before comparing.
; Returns: 0 if equal, <0 if ptr1<ptr2, >0 if ptr1>ptr2
; ==================================================================
er_fn er_memicmp
    er_check_zero rdx, .memicmp_equal
    xor     r8d, r8d
.memicmp_loop:
    cmp     r8, rdx
    jae     .memicmp_equal
    mov     al, byte [rdi + r8]
    mov     cl, byte [rsi + r8]
    cmp     al, 'A'
    jb      .memicmp_fold_cl
    cmp     al, 'Z'
    ja      .memicmp_fold_cl
    add     al, 32
.memicmp_fold_cl:
    cmp     cl, 'A'
    jb      .memicmp_compare
    cmp     cl, 'Z'
    ja      .memicmp_compare
    add     cl, 32
.memicmp_compare:
    cmp     al, cl
    jne     .memicmp_diff
    inc     r8
    jmp     .memicmp_loop
.memicmp_diff:
    movzx   eax, al
    movzx   ecx, cl
    sub     eax, ecx
    er_ret
.memicmp_equal:
    xor     eax, eax
    er_ret

; ==================================================================
; er_memswap — swap two equal-length memory buffers
; void er_memswap(void* ptr1, void* ptr2, size_t n)
; Swaps in 8-byte chunks, then byte-by-byte remainder.
; ==================================================================
er_fn er_memswap
    er_check_zero rdx, .memswap_done
    mov     rcx, rdx
    shr     rcx, 3
.memswap_8_loop:
    er_check_zero rcx, .memswap_remain
    mov     r8, [rdi]
    mov     r9, [rsi]
    mov     [rdi], r9
    mov     [rsi], r8
    add     rdi, 8
    add     rsi, 8
    dec     rcx
    jmp     .memswap_8_loop
.memswap_remain:
    mov     rcx, rdx
    and     rcx, 7
.memswap_1_loop:
    er_check_zero rcx, .memswap_done
    mov     al, byte [rdi + rcx - 1]
    mov     r10b, byte [rsi + rcx - 1]
    mov     byte [rsi + rcx - 1], al
    mov     byte [rdi + rcx - 1], r10b
    dec     rcx
    jmp     .memswap_1_loop
.memswap_done:
    er_ret
