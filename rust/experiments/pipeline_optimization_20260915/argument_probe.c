/* SPDX-License-Identifier: GPL-3.0-only
 * Diagnostic only: inspect public synthetic smoke arguments, never data. */
#include "stplugin.h"
#include <stdio.h>
#include <string.h>

STDLL stata_call(int argc, char *argv[])
{
    char message[1024];
    for (int i = 0; i < argc; ++i) {
        (void)snprintf(message, sizeof(message), "ARGPROBE %d %zu <%s>\n", i, strlen(argv[i]), argv[i]);
        SF_display(message);
    }
    return 0;
}
