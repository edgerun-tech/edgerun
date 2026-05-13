use crate::protocol::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeRole(u16);

impl NodeRole {
    pub const RELAY: Self = Self(NODE_ROLE_RELAY);
    pub const STORAGE: Self = Self(NODE_ROLE_STORAGE);
    pub const COMPUTE: Self = Self(NODE_ROLE_COMPUTE);
    pub const ADMISSION: Self = Self(NODE_ROLE_ADMISSION);
    pub const MESSAGE: Self = Self(NODE_ROLE_MESSAGE);

    pub const fn as_u16(self) -> u16 {
        self.0
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            NODE_ROLE_RELAY | NODE_ROLE_STORAGE | NODE_ROLE_COMPUTE | NODE_ROLE_ADMISSION
            | NODE_ROLE_MESSAGE => Some(Self(value)),
            _ => None,
        }
    }

    pub fn is_valid(value: u16) -> bool {
        Self::from_u16(value).is_some()
    }
}

impl From<NodeRole> for u16 {
    fn from(value: NodeRole) -> Self {
        value.as_u16()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkType(u16);

impl WorkType {
    pub const MESSAGE_DELIVER: Self = Self(WORK_TYPE_MESSAGE_DELIVER);
    pub const OBJECT_STORE: Self = Self(WORK_TYPE_OBJECT_STORE);
    pub const OBJECT_RETRIEVE: Self = Self(WORK_TYPE_OBJECT_RETRIEVE);
    pub const OBJECT_PIN: Self = Self(WORK_TYPE_OBJECT_PIN);
    pub const COMPUTE_RUN: Self = Self(WORK_TYPE_COMPUTE_RUN);
    pub const PROGRAM_OPEN: Self = Self(WORK_TYPE_PROGRAM_OPEN);
    pub const PROGRAM_STDIN: Self = Self(WORK_TYPE_PROGRAM_STDIN);
    pub const PROGRAM_CLOSE: Self = Self(WORK_TYPE_PROGRAM_CLOSE);
    pub const PROGRAM_POLL: Self = Self(WORK_TYPE_PROGRAM_POLL);
    pub const PROGRAM_EVENT: Self = Self(WORK_TYPE_PROGRAM_EVENT);

    pub const fn as_u16(self) -> u16 {
        self.0
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            WORK_TYPE_MESSAGE_DELIVER
            | WORK_TYPE_OBJECT_STORE
            | WORK_TYPE_OBJECT_RETRIEVE
            | WORK_TYPE_OBJECT_PIN
            | WORK_TYPE_COMPUTE_RUN
            | WORK_TYPE_PROGRAM_OPEN
            | WORK_TYPE_PROGRAM_STDIN
            | WORK_TYPE_PROGRAM_CLOSE
            | WORK_TYPE_PROGRAM_POLL
            | WORK_TYPE_PROGRAM_EVENT => Some(Self(value)),
            _ => None,
        }
    }

    pub fn is_valid(value: u16) -> bool {
        Self::from_u16(value).is_some()
    }
}

impl From<WorkType> for u16 {
    fn from(value: WorkType) -> Self {
        value.as_u16()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Department(u16);

impl Department {
    pub const ADMISSION: Self = Self(DEPARTMENT_ADMISSION);
    pub const RELAY: Self = Self(DEPARTMENT_RELAY);
    pub const MESSAGE: Self = Self(DEPARTMENT_MESSAGE);
    pub const STORAGE: Self = Self(DEPARTMENT_STORAGE);
    pub const RETRIEVAL: Self = Self(DEPARTMENT_RETRIEVAL);
    pub const COMPUTE: Self = Self(DEPARTMENT_COMPUTE);

    pub const fn as_u16(self) -> u16 {
        self.0
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            DEPARTMENT_ADMISSION | DEPARTMENT_RELAY | DEPARTMENT_MESSAGE | DEPARTMENT_STORAGE
            | DEPARTMENT_RETRIEVAL | DEPARTMENT_COMPUTE => Some(Self(value)),
            _ => None,
        }
    }

    pub fn is_valid(value: u16) -> bool {
        Self::from_u16(value).is_some()
    }
}

impl From<Department> for u16 {
    fn from(value: Department) -> Self {
        value.as_u16()
    }
}

pub fn department_for_work_type_typed(work_type: WorkType) -> Option<Department> {
    match work_type.as_u16() {
        WORK_TYPE_MESSAGE_DELIVER => Some(Department::MESSAGE),
        WORK_TYPE_OBJECT_STORE | WORK_TYPE_OBJECT_PIN => Some(Department::STORAGE),
        WORK_TYPE_OBJECT_RETRIEVE => Some(Department::RETRIEVAL),
        WORK_TYPE_COMPUTE_RUN
        | WORK_TYPE_PROGRAM_OPEN
        | WORK_TYPE_PROGRAM_STDIN
        | WORK_TYPE_PROGRAM_CLOSE
        | WORK_TYPE_PROGRAM_POLL
        | WORK_TYPE_PROGRAM_EVENT => Some(Department::COMPUTE),
        _ => None,
    }
}

pub fn role_for_department_typed(department: Department) -> Option<NodeRole> {
    match department.as_u16() {
        DEPARTMENT_RELAY => Some(NodeRole::RELAY),
        DEPARTMENT_MESSAGE => Some(NodeRole::MESSAGE),
        DEPARTMENT_STORAGE | DEPARTMENT_RETRIEVAL => Some(NodeRole::STORAGE),
        DEPARTMENT_COMPUTE => Some(NodeRole::COMPUTE),
        DEPARTMENT_ADMISSION => Some(NodeRole::ADMISSION),
        _ => None,
    }
}
