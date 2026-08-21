/* SPDX-License-Identifier: GPL-3.0-only */

#include "stata_interrupt.h"

#include <assert.h>

ST_plugin *_stata_;

static ST_int mock_stopflag;
static ST_int mock_poll_result;
static ST_int mock_set_stopflag;
static unsigned int mock_poll_calls;

static ST_int mock_poll(void)
{
    ++mock_poll_calls;
    if (mock_set_stopflag != 0) {
        mock_stopflag = 1;
    }
    return mock_poll_result;
}

static void reset_mock(void)
{
    mock_stopflag = 0;
    mock_poll_result = 0;
    mock_set_stopflag = 0;
    mock_poll_calls = 0;
}

int main(void)
{
    ST_plugin plugin = {0};
    plugin.pollstd = mock_poll;
    plugin.stopflag = &mock_stopflag;
    _stata_ = &plugin;

    reset_mock();
    assert(vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_CONTINUE);
    assert(mock_poll_calls == 1);

    reset_mock();
    mock_poll_result = 1;
    assert(vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK);
    assert(mock_poll_calls == 1);

    reset_mock();
    mock_stopflag = 1;
    assert(vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK);
    assert(mock_poll_calls == 0);

    reset_mock();
    mock_set_stopflag = 1;
    assert(vckss_stata_interrupt_poll(NULL) == VCKSS_INTERRUPT_USER_BREAK);
    assert(mock_poll_calls == 1);
    return 0;
}
