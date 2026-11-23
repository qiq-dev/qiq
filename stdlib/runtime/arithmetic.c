// Integer arithmetic that needs checks. ints are 64-bit; overflow wraps (two's
// complement), matching `+`, `-`, and `*`; division by zero is an error.
#include <stdint.h>

#include "qiq_runtime.h"

// Truncates toward zero. INT64_MIN / -1 wraps to INT64_MIN.
QIQ_GUARDED(int64_t, qiq_int_divide, 0, (int64_t left, int64_t right), (left, right)) {
    if (right == 0) qiq_throw(QIQ_DIVISION_BY_ZERO_EXCEPTION, "division by zero");
    if (left == INT64_MIN && right == -1) return INT64_MIN;
    return left / right;
}

// The remainder has the sign of `left`. INT64_MIN % -1 is 0.
QIQ_GUARDED(int64_t, qiq_int_modulo, 0, (int64_t left, int64_t right), (left, right)) {
    if (right == 0) qiq_throw(QIQ_DIVISION_BY_ZERO_EXCEPTION, "modulo by zero");
    if (right == -1) return 0;
    return left % right;
}

// Exponentiation by squaring, wrapping on overflow, for an exponent the
// compiled code has checked isn't negative. Unguarded, so LLVM can inline it.
int64_t qiq_int_power_unchecked(int64_t base, int64_t exponent) {
    uint64_t result = 1;
    uint64_t factor = (uint64_t)base;
    uint64_t remaining = (uint64_t)exponent;
    while (remaining > 0) {
        if (remaining & 1) result *= factor;
        factor *= factor;
        remaining >>= 1;
    }
    return (int64_t)result;
}

// The same, throwing for a negative exponent.
QIQ_GUARDED(int64_t, qiq_int_power, 0, (int64_t base, int64_t exponent), (base, exponent)) {
    if (exponent < 0) qiq_throw(QIQ_ARITHMETIC_EXCEPTION, "int exponent must not be negative; use doubles for fractional powers");
    return qiq_int_power_unchecked(base, exponent);
}

// Shifts follow PHP: a negative amount is an error, and shifting by the full
// width (64) or more gives 0 (or -1 when shifting a negative number right).
QIQ_GUARDED(int64_t, qiq_int_shift_left, 0, (int64_t value, int64_t amount), (value, amount)) {
    if (amount < 0) qiq_throw(QIQ_ARITHMETIC_EXCEPTION, "shift amount must not be negative");
    if (amount >= 64) return 0;
    return (int64_t)((uint64_t)value << amount);
}

// Arithmetic shift: the sign bit is copied in from the left.
QIQ_GUARDED(int64_t, qiq_int_shift_right, 0, (int64_t value, int64_t amount), (value, amount)) {
    if (amount < 0) qiq_throw(QIQ_ARITHMETIC_EXCEPTION, "shift amount must not be negative");
    if (amount >= 64) return value < 0 ? -1 : 0;
    return value < 0 ? ~(~value >> amount) : value >> amount;
}
