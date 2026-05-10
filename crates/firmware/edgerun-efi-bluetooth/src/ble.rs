use crate::hci::{self, Hci};
use crate::uefi::{EFI_INVALID_PARAMETER, EfiStatus};

const OCF_LE_SET_EVENT_MASK: u16 = 0x0001;
const OCF_LE_SET_ADVERTISING_PARAMETERS: u16 = 0x0006;
const OCF_LE_SET_ADVERTISING_DATA: u16 = 0x0008;
const OCF_LE_SET_SCAN_RESPONSE_DATA: u16 = 0x0009;
const OCF_LE_SET_ADVERTISE_ENABLE: u16 = 0x000a;
const OCF_LE_SET_SCAN_PARAMETERS: u16 = 0x000b;
const OCF_LE_SET_SCAN_ENABLE: u16 = 0x000c;

const HCI_LE_SET_EVENT_MASK: u16 = hci::hci_opcode(hci::OGF_LE_CONTROLLER, OCF_LE_SET_EVENT_MASK);
const HCI_LE_SET_ADV_PARAMS: u16 =
    hci::hci_opcode(hci::OGF_LE_CONTROLLER, OCF_LE_SET_ADVERTISING_PARAMETERS);
const HCI_LE_SET_ADV_DATA: u16 =
    hci::hci_opcode(hci::OGF_LE_CONTROLLER, OCF_LE_SET_ADVERTISING_DATA);
const HCI_LE_SET_SCAN_RSP_DATA: u16 =
    hci::hci_opcode(hci::OGF_LE_CONTROLLER, OCF_LE_SET_SCAN_RESPONSE_DATA);
const HCI_LE_SET_ADV_ENABLE: u16 =
    hci::hci_opcode(hci::OGF_LE_CONTROLLER, OCF_LE_SET_ADVERTISE_ENABLE);
const HCI_LE_SET_SCAN_PARAMETERS: u16 =
    hci::hci_opcode(hci::OGF_LE_CONTROLLER, OCF_LE_SET_SCAN_PARAMETERS);
const HCI_LE_SET_SCAN_ENABLE: u16 =
    hci::hci_opcode(hci::OGF_LE_CONTROLLER, OCF_LE_SET_SCAN_ENABLE);

pub fn start_advertising(hci: &mut Hci, local_name: &[u8]) -> Result<(), EfiStatus> {
    if local_name.is_empty() || local_name.len() > 24 {
        return Err(EFI_INVALID_PARAMETER);
    }

    let mut event = [0u8; 260];

    let le_event_mask = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x1f];
    hci.command(HCI_LE_SET_EVENT_MASK, &le_event_mask, &mut event)?;

    let adv_params = [
        0xa0, 0x00, // 100 ms minimum interval
        0xa0, 0x00, // 100 ms maximum interval
        0x00,       // ADV_IND connectable undirected
        0x00,       // own public address
        0x00,       // direct public address type
        0, 0, 0, 0, 0, 0, // direct address unused
        0x07,       // channels 37, 38, 39
        0x00,       // allow all scanners/connectors
    ];
    hci.command(HCI_LE_SET_ADV_PARAMS, &adv_params, &mut event)?;

    let mut adv_data = [0u8; 32];
    let mut pos = 1usize;

    // Flags: LE General Discoverable Mode + BR/EDR Not Supported.
    adv_data[pos] = 0x02;
    adv_data[pos + 1] = 0x01;
    adv_data[pos + 2] = 0x06;
    pos += 3;

    // Complete Local Name.
    adv_data[pos] = (1 + local_name.len()) as u8;
    adv_data[pos + 1] = 0x09;
    let mut i = 0usize;
    while i < local_name.len() {
        adv_data[pos + 2 + i] = local_name[i];
        i += 1;
    }
    pos += 2 + local_name.len();
    adv_data[0] = (pos - 1) as u8;

    hci.command(HCI_LE_SET_ADV_DATA, &adv_data, &mut event)?;

    let scan_rsp = [0u8; 32];
    hci.command(HCI_LE_SET_SCAN_RSP_DATA, &scan_rsp, &mut event)?;

    hci.command(HCI_LE_SET_ADV_ENABLE, &[0x01], &mut event)?;
    Ok(())
}

pub fn start_passive_scanning(hci: &mut Hci) -> Result<(), EfiStatus> {
    let mut event = [0u8; 260];
    let scan_params = [
        0x00,       // passive scan
        0x60, 0x00, // interval: 60 ms
        0x30, 0x00, // window: 30 ms
        0x00,       // own public address
        0x00,       // accept all advertisements
    ];
    hci.command(HCI_LE_SET_SCAN_PARAMETERS, &scan_params, &mut event)?;
    hci.command(HCI_LE_SET_SCAN_ENABLE, &[0x01, 0x00], &mut event)?;
    Ok(())
}
