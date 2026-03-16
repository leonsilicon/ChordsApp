#include <stdio.h>
#include <stdbool.h>
#include <mach/mach.h>
#include <IOKit/IOKitLib.h>
#include <IOKit/hid/IOHIDBase.h>
#include <IOKit/hidsystem/IOHIDLib.h>

static io_connect_t open_hid_system(void) {
    io_service_t service =
        IOServiceGetMatchingService(
            kIOMasterPortDefault,
            IOServiceMatching(kIOHIDSystemClass)
        );

    if (!service) {
        fprintf(stderr, "Failed to find IOHIDSystem\n");
        return MACH_PORT_NULL;
    }

    io_connect_t conn = MACH_PORT_NULL;
    kern_return_t kr =
        IOServiceOpen(service, mach_task_self(), kIOHIDParamConnectType, &conn);

    IOObjectRelease(service);

    if (kr != KERN_SUCCESS) {
        fprintf(stderr, "IOServiceOpen failed: 0x%x\n", kr);
        return MACH_PORT_NULL;
    }

    return conn;
}

int get_caps_state(int *out_state) {
    if (!out_state) {
        return 1;
    }

    io_connect_t conn = open_hid_system();
    if (conn == MACH_PORT_NULL) {
        return 2;
    }

    bool state = false;
    kern_return_t kr = IOHIDGetModifierLockState(conn, kIOHIDCapsLockState, &state);
    IOServiceClose(conn);

    if (kr != KERN_SUCCESS) {
        fprintf(stderr, "IOHIDGetModifierLockState failed: 0x%x\n", kr);
        return 3;
    }

    *out_state = state ? 1 : 0;
    return 0;
}

static int set_caps_state(bool enabled) {
    io_connect_t conn = open_hid_system();
    if (conn == MACH_PORT_NULL) {
        return 2;
    }

    kern_return_t kr =
        IOHIDSetModifierLockState(conn, kIOHIDCapsLockState, enabled);

    IOServiceClose(conn);

    if (kr != KERN_SUCCESS) {
        fprintf(stderr, "IOHIDSetModifierLockState failed: 0x%x\n", kr);
        return 3;
    }

    return 0;
}

int set_caps_on(void) {
    return set_caps_state(true);
}

int set_caps_off(void) {
    return set_caps_state(false);
}

int toggle_caps(void) {
    int state = 0;
    int rc = get_caps_state(&state);
    if (rc != 0) {
        return rc;
    }
    return set_caps_state(state == 0);
}
