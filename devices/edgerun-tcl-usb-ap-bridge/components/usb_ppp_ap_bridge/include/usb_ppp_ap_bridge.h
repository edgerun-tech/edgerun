#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int edgerun_usb_ppp_ap_bridge_start(const char *ssid, const char *password, uint8_t channel);
int edgerun_usb_ppp_ap_bridge_status(void);

#ifdef __cplusplus
}
#endif
