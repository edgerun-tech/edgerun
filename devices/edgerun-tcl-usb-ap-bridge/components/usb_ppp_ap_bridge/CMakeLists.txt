idf_component_register(
    SRCS "usb_ppp_ap_bridge.c"
    INCLUDE_DIRS
        "include"
        "${IDF_PATH}/components/freertos/FreeRTOS-Kernel/include/freertos"
        "${IDF_PATH}/components/freertos/FreeRTOS-Kernel-SMP/include/freertos"
    PRIV_REQUIRES esp_wifi esp_netif nvs_flash esp_event esp_tinyusb
)
