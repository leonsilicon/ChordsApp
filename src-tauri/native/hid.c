#include <IOKit/hid/IOHIDManager.h>
#include <CoreFoundation/CoreFoundation.h>
#include <stdio.h>

typedef void (*capslock_callback)(int pressed);

static capslock_callback rust_callback = NULL;

static void input_callback(
    void *context,
    IOReturn result,
    void *sender,
    IOHIDValueRef value
) {
    (void)context;
    (void)result;
    (void)sender;

    IOHIDElementRef element = IOHIDValueGetElement(value);

    uint32_t usage_page = IOHIDElementGetUsagePage(element);
    uint32_t usage = IOHIDElementGetUsage(element);

    // keyboard page
    if (usage_page != 0x07)
        return;

    // caps lock
    if (usage != 0x39)
        return;

    int pressed = IOHIDValueGetIntegerValue(value);

    if (rust_callback) {
        rust_callback(pressed);
    }
}

void start_caps_lock_listener(capslock_callback cb) {
    rust_callback = cb;

    IOHIDManagerRef manager =
        IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone);

    // Match keyboard devices only
    CFMutableDictionaryRef matching =
        CFDictionaryCreateMutable(
            kCFAllocatorDefault,
            0,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks
        );

    int page = kHIDPage_GenericDesktop;
    int usage = kHIDUsage_GD_Keyboard;

    CFNumberRef page_ref =
        CFNumberCreate(kCFAllocatorDefault, kCFNumberIntType, &page);

    CFNumberRef usage_ref =
        CFNumberCreate(kCFAllocatorDefault, kCFNumberIntType, &usage);

    CFDictionarySetValue(
        matching,
        CFSTR(kIOHIDDeviceUsagePageKey),
        page_ref
    );

    CFDictionarySetValue(
        matching,
        CFSTR(kIOHIDDeviceUsageKey),
        usage_ref
    );

    IOHIDManagerSetDeviceMatching(manager, matching);

    IOHIDManagerRegisterInputValueCallback(
        manager,
        input_callback,
        NULL
    );

    IOHIDManagerScheduleWithRunLoop(
        manager,
        CFRunLoopGetCurrent(),
        kCFRunLoopDefaultMode
    );

    IOHIDManagerOpen(manager, kIOHIDOptionsTypeNone);

    printf("Caps Lock HID listener started\n");

    CFRunLoopRun();
}
