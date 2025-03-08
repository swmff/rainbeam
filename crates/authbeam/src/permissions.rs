use serde::{
    de::{Error as DeError, Visitor},
    Deserialize, Deserializer, Serialize,
};
use bitflags::bitflags;

bitflags! {
    /// Fine-grained permissions built using bitwise operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FinePermission: u32 {
        const DEFAULT = 1 << 0;
        const ADMINISTRATOR = 1 << 1;
        const MANAGE_RESPONSES = 1 << 2;
        const MANAGE_COMMENTS = 1 << 3;
        const MANAGE_QUESTIONS = 1 << 4;
        const MANAGE_CHATS = 1 << 5;
        const MANAGE_MESSAGES = 1 << 6;
        const MANAGE_MAILS = 1 << 7;
        const MANAGE_NOTIFICATIONS = 1 << 8;
        const MANAGE_CIRCLES = 1 << 9;
        const BAN_IP = 1 << 10;
        const UNBAN_IP = 1 << 11;
        const APPROVE_MARKETPLACE_ITEMS = 1 << 12;
        const VIEW_PROFILE_MANAGE = 1 << 13;
        const VIEW_AUDIT_LOG = 1 << 14;
        const VIEW_REPORTS = 1 << 15;
        const MANAGE_PROFILE_GROUP = 1 << 16;
        const MANAGE_PROFILE_TIER = 1 << 17;
        const MANAGE_PROFILE_SETTINGS = 1 << 18;
        const MANAGE_GROUP_PERMISSIONS = 1 << 19;
        const PROMOTE_USERS = 1 << 20;
        const EDIT_USER = 1 << 21;
        const ECON_MASTER = 1 << 22;
        const DELETE_USER = 1 << 23;
        const CREATE_LABEL = 1 << 24;
        const MANAGE_WARNINGS = 1 << 25;
        const MANAGE_REACTIONS = 1 << 26;
        const EXPORT_DATA = 1 << 27;
        const MANAGE_LABELS = 1 << 28;

        const _ = !0;
    }
}

impl Serialize for FinePermission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.bits())
    }
}

struct FinePermissionVisitor;
impl<'de> Visitor<'de> for FinePermissionVisitor {
    type Value = FinePermission;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("u32")
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if let Some(permission) = FinePermission::from_bits(value) {
            Ok(permission)
        } else {
            Ok(FinePermission::from_bits_retain(value))
        }
    }

    fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if let Some(permission) = FinePermission::from_bits(value as u32) {
            Ok(permission)
        } else {
            Ok(FinePermission::from_bits_retain(value as u32))
        }
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if let Some(permission) = FinePermission::from_bits(value as u32) {
            Ok(permission)
        } else {
            Ok(FinePermission::from_bits_retain(value as u32))
        }
    }
}

impl<'de> Deserialize<'de> for FinePermission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(FinePermissionVisitor)
    }
}

impl FinePermission {
    /// Join two [`FinePermission`]s into a single `u32`.
    pub fn join(lhs: FinePermission, rhs: FinePermission) -> FinePermission {
        lhs | rhs
    }

    /// Check if the given `input` contains the given [`FinePermission`].
    pub fn check(self, permission: FinePermission) -> bool {
        if (self & FinePermission::ADMINISTRATOR) == FinePermission::ADMINISTRATOR {
            // has administrator permission, meaning everything else is automatically true
            return true;
        }

        (self & permission) == permission
    }

    /// Check if thhe given [`FinePermission`] is qualifies as "Helper" status.
    pub fn check_helper(self) -> bool {
        self.check(FinePermission::MANAGE_QUESTIONS)
            && self.check(FinePermission::MANAGE_RESPONSES)
            && self.check(FinePermission::MANAGE_COMMENTS)
            && self.check(FinePermission::MANAGE_WARNINGS)
            && self.check(FinePermission::VIEW_REPORTS)
            && self.check(FinePermission::VIEW_AUDIT_LOG)
    }

    /// Check if thhe given [`FinePermission`] is qualifies as "Manager" status.
    pub fn check_manager(self) -> bool {
        self.check_helper()
            && self.check(FinePermission::DELETE_USER)
            && self.check(FinePermission::BAN_IP)
            && self.check(FinePermission::UNBAN_IP)
            && self.check(FinePermission::VIEW_PROFILE_MANAGE)
    }
}

impl Default for FinePermission {
    fn default() -> Self {
        Self::DEFAULT
    }
}
