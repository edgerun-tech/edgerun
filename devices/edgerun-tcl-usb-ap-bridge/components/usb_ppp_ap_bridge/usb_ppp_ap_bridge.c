#include "usb_ppp_ap_bridge.h"

#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

#include "esp_check.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_netif.h"
#include "esp_netif_ppp.h"
#include "esp_wifi.h"
#include "freertos/FreeRTOS.h"
#include "freertos/event_groups.h"
#include "nvs_flash.h"
#include "sdkconfig.h"
#include "tinyusb.h"
#include "tinyusb_cdc_acm.h"
#include "tinyusb_default_config.h"

static const char *TAG = "edgerun_usb_ppp_ap";

static esp_netif_t *s_ppp_netif;
static esp_netif_t *s_ap_netif;
static EventGroupHandle_t s_events;
static int s_cdc_itf;
static uint8_t s_rx_buf[CONFIG_TINYUSB_CDC_RX_BUFSIZE];

static const int PPP_GOT_IP = BIT0;
static const int AP_STARTED = BIT1;

static esp_err_t ppp_transmit(void *handle, void *buffer, size_t len)
{
    (void)handle;
    tinyusb_cdcacm_write_queue(s_cdc_itf, buffer, len);
    tinyusb_cdcacm_write_flush(s_cdc_itf, 0);
    return ESP_OK;
}

static esp_netif_driver_ifconfig_t s_ppp_driver = {
    .handle = (void *)1,
    .transmit = ppp_transmit,
};

static void cdc_rx_callback(int itf, cdcacm_event_t *event)
{
    (void)event;
    size_t rx_size = 0;
    if (itf != s_cdc_itf || s_ppp_netif == NULL) {
        return;
    }

    esp_err_t ret = tinyusb_cdcacm_read(itf, s_rx_buf, CONFIG_TINYUSB_CDC_RX_BUFSIZE, &rx_size);
    if (ret == ESP_OK && rx_size > 0) {
        esp_netif_receive(s_ppp_netif, s_rx_buf, rx_size, NULL);
    }
}

static void cdc_line_state_callback(int itf, cdcacm_event_t *event)
{
    (void)event;
    s_cdc_itf = itf;
}

static void ip_event_handler(void *arg, esp_event_base_t event_base, int32_t event_id, void *event_data)
{
    (void)arg;
    (void)event_base;

    if (event_id == IP_EVENT_PPP_GOT_IP) {
        ip_event_got_ip_t *event = (ip_event_got_ip_t *)event_data;
        if (event->esp_netif != s_ppp_netif) {
            return;
        }
        esp_netif_set_default_netif(s_ppp_netif);
        if (s_ap_netif != NULL) {
            esp_err_t ret = esp_netif_napt_enable(s_ap_netif);
            if (ret != ESP_OK && ret != ESP_ERR_INVALID_STATE) {
                return;
            }
        }
        xEventGroupSetBits(s_events, PPP_GOT_IP);
    } else if (event_id == IP_EVENT_PPP_LOST_IP) {
        xEventGroupClearBits(s_events, PPP_GOT_IP);
    }
}

static void wifi_event_handler(void *arg, esp_event_base_t event_base, int32_t event_id, void *event_data)
{
    (void)arg;
    (void)event_base;
    if (event_id == WIFI_EVENT_AP_START) {
        xEventGroupSetBits(s_events, AP_STARTED);
    } else if (event_id == WIFI_EVENT_AP_STACONNECTED) {
        (void)event_data;
    } else if (event_id == WIFI_EVENT_AP_STADISCONNECTED) {
        (void)event_data;
    }
}

static esp_err_t init_nvs(void)
{
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_RETURN_ON_ERROR(nvs_flash_erase(), TAG, "nvs erase failed");
        ret = nvs_flash_init();
    }
    return ret;
}

static esp_err_t start_usb_ppp(void)
{
    const tinyusb_config_t tusb_cfg = TINYUSB_DEFAULT_CONFIG();
    ESP_RETURN_ON_ERROR(tinyusb_driver_install(&tusb_cfg), TAG, "tinyusb init failed");

    tinyusb_config_cdcacm_t acm_cfg = {
        .cdc_port = TINYUSB_CDC_ACM_0,
        .callback_rx = cdc_rx_callback,
        .callback_rx_wanted_char = NULL,
        .callback_line_state_changed = NULL,
        .callback_line_coding_changed = NULL,
    };
    ESP_RETURN_ON_ERROR(tinyusb_cdcacm_init(&acm_cfg), TAG, "cdc-acm init failed");
    ESP_RETURN_ON_ERROR(
        tinyusb_cdcacm_register_callback(TINYUSB_CDC_ACM_0, CDC_EVENT_LINE_STATE_CHANGED, cdc_line_state_callback),
        TAG,
        "cdc line callback failed");

    esp_netif_inherent_config_t base_netif_cfg = ESP_NETIF_INHERENT_DEFAULT_PPP();
    base_netif_cfg.if_desc = "edgerun_ppp";
    esp_netif_config_t ppp_cfg = {
        .base = &base_netif_cfg,
        .driver = &s_ppp_driver,
        .stack = ESP_NETIF_NETSTACK_DEFAULT_PPP,
    };
    s_ppp_netif = esp_netif_new(&ppp_cfg);
    assert(s_ppp_netif);
    esp_netif_action_start(s_ppp_netif, 0, 0, 0);
    esp_netif_action_connected(s_ppp_netif, 0, 0, 0);
    return ESP_OK;
}

static esp_err_t start_softap(const char *ssid, const char *password, uint8_t channel)
{
    s_ap_netif = esp_netif_create_default_wifi_ap();
    if (s_ap_netif == NULL) {
        return ESP_FAIL;
    }

    wifi_init_config_t wifi_cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_RETURN_ON_ERROR(esp_wifi_init(&wifi_cfg), TAG, "wifi init failed");
    ESP_RETURN_ON_ERROR(esp_event_handler_register(WIFI_EVENT, ESP_EVENT_ANY_ID, wifi_event_handler, NULL), TAG, "wifi handler failed");

    wifi_config_t ap_cfg = {0};
    strlcpy((char *)ap_cfg.ap.ssid, ssid, sizeof(ap_cfg.ap.ssid));
    strlcpy((char *)ap_cfg.ap.password, password, sizeof(ap_cfg.ap.password));
    ap_cfg.ap.ssid_len = strlen(ssid);
    ap_cfg.ap.channel = channel ? channel : 6;
    ap_cfg.ap.max_connection = 4;
    ap_cfg.ap.authmode = strlen(password) == 0 ? WIFI_AUTH_OPEN : WIFI_AUTH_WPA2_PSK;
    ap_cfg.ap.pmf_cfg.required = false;

    ESP_RETURN_ON_ERROR(esp_wifi_set_mode(WIFI_MODE_AP), TAG, "set AP mode failed");
    ESP_RETURN_ON_ERROR(esp_wifi_set_config(WIFI_IF_AP, &ap_cfg), TAG, "set AP config failed");
    ESP_RETURN_ON_ERROR(esp_wifi_start(), TAG, "wifi start failed");
    return ESP_OK;
}

int edgerun_usb_ppp_ap_bridge_start(const char *ssid, const char *password, uint8_t channel)
{
    if (ssid == NULL || ssid[0] == '\0') {
        return ESP_ERR_INVALID_ARG;
    }
    if (password == NULL) {
        password = "";
    }

    ESP_ERROR_CHECK(init_nvs());
    ESP_ERROR_CHECK(esp_netif_init());
    esp_err_t loop_ret = esp_event_loop_create_default();
    if (loop_ret != ESP_OK && loop_ret != ESP_ERR_INVALID_STATE) {
        ESP_ERROR_CHECK(loop_ret);
    }

    s_events = xEventGroupCreate();
    if (s_events == NULL) {
        return ESP_ERR_NO_MEM;
    }

    ESP_ERROR_CHECK(esp_event_handler_register(IP_EVENT, ESP_EVENT_ANY_ID, ip_event_handler, NULL));
    ESP_ERROR_CHECK(start_usb_ppp());
    ESP_ERROR_CHECK(start_softap(ssid, password, channel));

    return ESP_OK;
}

int edgerun_usb_ppp_ap_bridge_status(void)
{
    if (s_events == NULL) {
        return 0;
    }
    EventBits_t bits = xEventGroupGetBits(s_events);
    int status = 0;
    if (bits & AP_STARTED) {
        status |= 1;
    }
    if (bits & PPP_GOT_IP) {
        status |= 2;
    }
    return status;
}
