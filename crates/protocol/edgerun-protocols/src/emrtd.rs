use alloc::vec;
use alloc::vec::Vec;

pub const EMRTD_AID: &[u8] = &[0xA0, 0x00, 0x00, 0x02, 0x47, 0x10, 0x01];
pub const EF_COM: u16 = 0x011E;
pub const EF_SOD: u16 = 0x011D;
pub const EF_CARD_ACCESS: u16 = 0x011C;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataGroup {
    Dg1,
    Dg2,
    Dg3,
    Dg4,
    Dg5,
    Dg6,
    Dg7,
    Dg8,
    Dg9,
    Dg10,
    Dg11,
    Dg12,
    Dg13,
    Dg14,
    Dg15,
    Dg16,
}

impl DataGroup {
    pub fn file_id(self) -> u16 {
        match self {
            DataGroup::Dg1 => 0x0101,
            DataGroup::Dg2 => 0x0102,
            DataGroup::Dg3 => 0x0103,
            DataGroup::Dg4 => 0x0104,
            DataGroup::Dg5 => 0x0105,
            DataGroup::Dg6 => 0x0106,
            DataGroup::Dg7 => 0x0107,
            DataGroup::Dg8 => 0x0108,
            DataGroup::Dg9 => 0x0109,
            DataGroup::Dg10 => 0x010A,
            DataGroup::Dg11 => 0x010B,
            DataGroup::Dg12 => 0x010C,
            DataGroup::Dg13 => 0x010D,
            DataGroup::Dg14 => 0x010E,
            DataGroup::Dg15 => 0x010F,
            DataGroup::Dg16 => 0x0110,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0x61 => Some(DataGroup::Dg1),
            0x75 => Some(DataGroup::Dg2),
            0x63 => Some(DataGroup::Dg3),
            0x76 => Some(DataGroup::Dg4),
            0x65 => Some(DataGroup::Dg5),
            0x66 => Some(DataGroup::Dg6),
            0x67 => Some(DataGroup::Dg7),
            0x68 => Some(DataGroup::Dg8),
            0x69 => Some(DataGroup::Dg9),
            0x6A => Some(DataGroup::Dg10),
            0x6B => Some(DataGroup::Dg11),
            0x6C => Some(DataGroup::Dg12),
            0x6D => Some(DataGroup::Dg13),
            0x6E => Some(DataGroup::Dg14),
            0x6F => Some(DataGroup::Dg15),
            0x70 => Some(DataGroup::Dg16),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusWord(pub u8, pub u8);

impl StatusWord {
    pub const OK: Self = Self(0x90, 0x00);

    pub fn is_ok(self) -> bool {
        self == Self::OK
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmrtdError {
    MalformedApdu,
    MalformedResponse,
    Status(StatusWord),
}

pub type Result<T> = core::result::Result<T, EmrtdError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedApdu<'a> {
    pub cla: u8,
    pub ins: u8,
    pub p1: u8,
    pub p2: u8,
    pub data: &'a [u8],
    pub le: Option<u8>,
}

impl<'a> ParsedApdu<'a> {
    pub fn parse(command: &'a [u8]) -> Result<Self> {
        if command.len() < 4 {
            return Err(EmrtdError::MalformedApdu);
        }
        let mut out = ParsedApdu {
            cla: command[0],
            ins: command[1],
            p1: command[2],
            p2: command[3],
            data: &[],
            le: None,
        };
        match command.len() {
            4 => Ok(out),
            5 => {
                out.le = Some(command[4]);
                Ok(out)
            }
            _ => {
                let lc = command[4] as usize;
                if command.len() == 5 + lc {
                    out.data = &command[5..5 + lc];
                    Ok(out)
                } else if command.len() == 6 + lc {
                    out.data = &command[5..5 + lc];
                    out.le = Some(command[5 + lc]);
                    Ok(out)
                } else {
                    Err(EmrtdError::MalformedApdu)
                }
            }
        }
    }
}

pub fn select_by_name(aid: &[u8]) -> Vec<u8> {
    let mut out = vec![0x00, 0xA4, 0x04, 0x0C, aid.len() as u8];
    out.extend_from_slice(aid);
    out
}

pub fn select_file(fid: u16) -> Vec<u8> {
    vec![0x00, 0xA4, 0x02, 0x0C, 0x02, (fid >> 8) as u8, fid as u8]
}

pub fn read_binary(offset: u16, le: u8) -> Vec<u8> {
    vec![0x00, 0xB0, (offset >> 8) as u8, offset as u8, le]
}

pub fn get_challenge() -> Vec<u8> {
    vec![0x00, 0x84, 0x00, 0x00, 0x08]
}

pub fn mutual_authenticate(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x00, 0x82, 0x00, 0x00, data.len() as u8];
    out.extend_from_slice(data);
    out.push(0x28);
    out
}

pub fn split_response(response: &[u8]) -> Result<Vec<u8>> {
    if response.len() < 2 {
        return Err(EmrtdError::MalformedResponse);
    }
    let data_len = response.len() - 2;
    let status = StatusWord(response[data_len], response[data_len + 1]);
    if !status.is_ok() {
        return Err(EmrtdError::Status(status));
    }
    Ok(response[..data_len].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_standard_apdus() {
        assert_eq!(
            select_by_name(EMRTD_AID),
            vec![
                0x00, 0xA4, 0x04, 0x0C, 0x07, 0xA0, 0x00, 0x00, 0x02, 0x47, 0x10, 0x01
            ]
        );
        assert_eq!(
            select_file(EF_COM),
            vec![0x00, 0xA4, 0x02, 0x0C, 0x02, 0x01, 0x1E]
        );
        assert_eq!(
            read_binary(0x0102, 0x20),
            vec![0x00, 0xB0, 0x01, 0x02, 0x20]
        );
        assert_eq!(get_challenge(), vec![0x00, 0x84, 0x00, 0x00, 0x08]);
    }

    #[test]
    fn parses_short_apdus() {
        let command = read_binary(0, 8);
        let parsed = ParsedApdu::parse(&command).unwrap();
        assert_eq!(parsed.cla, 0x00);
        assert_eq!(parsed.ins, 0xB0);
        assert_eq!(parsed.p1, 0x00);
        assert_eq!(parsed.p2, 0x00);
        assert!(parsed.data.is_empty());
        assert_eq!(parsed.le, Some(8));

        let command = mutual_authenticate(&[1, 2, 3, 4]);
        let parsed = ParsedApdu::parse(&command).unwrap();
        assert_eq!(parsed.data, &[1, 2, 3, 4]);
        assert_eq!(parsed.le, Some(0x28));
    }

    #[test]
    fn splits_status_word_response() {
        assert_eq!(split_response(&[1, 2, 0x90, 0x00]).unwrap(), vec![1, 2]);
        assert_eq!(
            split_response(&[0x6A, 0x82]),
            Err(EmrtdError::Status(StatusWord(0x6A, 0x82)))
        );
        assert_eq!(split_response(&[0x90]), Err(EmrtdError::MalformedResponse));
    }

    #[test]
    fn maps_data_groups() {
        assert_eq!(DataGroup::Dg1.file_id(), 0x0101);
        assert_eq!(DataGroup::Dg16.file_id(), 0x0110);
        assert_eq!(DataGroup::from_tag(0x61), Some(DataGroup::Dg1));
        assert_eq!(DataGroup::from_tag(0x70), Some(DataGroup::Dg16));
        assert_eq!(DataGroup::from_tag(0), None);
    }
}
