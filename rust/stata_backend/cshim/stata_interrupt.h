/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef VCKSS_STATA_INTERRUPT_H
#define VCKSS_STATA_INTERRUPT_H

#include "stplugin.h"
#include "vckss_rust.h"

/* This helper is entered synchronously from Rust on the same thread that
   entered stata_call(). No Stata function-table pointer crosses into Rust. */
static inline int32_t vckss_stata_interrupt_poll(void *context)
{
    ST_int poll_status;
    (void)context;
    if (SW_stopflag != 0) {
        return VCKSS_INTERRUPT_USER_BREAK;
    }
    poll_status = SF_poll();
    return poll_status != 0 || SW_stopflag != 0
        ? VCKSS_INTERRUPT_USER_BREAK
        : VCKSS_INTERRUPT_CONTINUE;
}

#endif /* VCKSS_STATA_INTERRUPT_H */
