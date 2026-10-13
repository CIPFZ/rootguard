//! 内容 id（SPEC 4.5）：定长内联字符串，`Copy`、按字典序比较，可直接放进组件与事件。

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 内联短字符串的最大字节数（ADR-002）。
pub const SHORT_STR_CAP: usize = 23;

/// id 构造失败。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// 超过 [`SHORT_STR_CAP`] 字节。
    #[error("id `{0}` 超过 {SHORT_STR_CAP} 字节")]
    TooLong(String),
    /// 含非 ASCII 字符。
    #[error("id `{0}` 含非 ASCII 字符")]
    NotAscii(String),
}

/// 最多 [`SHORT_STR_CAP`] 字节的 ASCII 字符串，`Copy`，不分配内存（ADR-002）。默认值为空串。
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ShortStr {
    len: u8,
    bytes: [u8; SHORT_STR_CAP],
}

impl ShortStr {
    /// 由 ASCII 字符串构造。
    ///
    /// # Errors
    ///
    /// 超长或含非 ASCII 字符时返回 [`IdError`]。
    pub fn new(s: &str) -> Result<Self, IdError> {
        if !s.is_ascii() {
            return Err(IdError::NotAscii(s.to_owned()));
        }
        let len = u8::try_from(s.len())
            .ok()
            .filter(|&n| usize::from(n) <= SHORT_STR_CAP)
            .ok_or_else(|| IdError::TooLong(s.to_owned()))?;
        let mut bytes = [0; SHORT_STR_CAP];
        bytes[..s.len()].copy_from_slice(s.as_bytes());
        Ok(Self { len, bytes })
    }

    /// 字符串视图。
    #[must_use]
    pub fn as_str(&self) -> &str {
        // 只能由 ASCII 字符串构造，切片必为合法 UTF-8
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl PartialOrd for ShortStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ShortStr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl fmt::Debug for ShortStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for ShortStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ShortStr {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ShortStr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::new(&s).map_err(serde::de::Error::custom)
    }
}

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub ShortStr);

        impl $name {
            /// 由字符串构造（只检查长度与 ASCII；字符集由内容校验负责，SPEC 4.5）。
            ///
            /// # Errors
            ///
            /// 超长或含非 ASCII 字符时返回 [`IdError`]。
            pub fn new(s: &str) -> Result<Self, IdError> {
                ShortStr::new(s).map(Self)
            }

            /// 字符串视图。
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:?})", stringify!($name), self.as_str())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

id_type!(
    /// 防御者 id，如 `pebble_pod`（SPEC 7.1）。
    DefenderId
);
id_type!(
    /// 敌人 id，如 `husk`（SPEC 8.1）。
    EnemyId
);
id_type!(
    /// 弹丸 id，如 `pebble`（SPEC 4.2）。
    ProjectileId
);
id_type!(
    /// 关卡 id，如 `1-3`（SPEC 10.2；格式见 ADR-002）。
    LevelId
);

impl LevelId {
    /// 关卡排序键：`"章-序"` 解析为 `(章, 序)`；格式不符时排在最后。
    #[must_use]
    pub fn order_key(&self) -> (u32, u32) {
        let mut parts = self.as_str().splitn(2, '-');
        let mut num = || parts.next().and_then(|p| p.parse::<u32>().ok());
        match (num(), num()) {
            (Some(a), Some(b)) => (a, b),
            _ => (u32::MAX, u32::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_str_roundtrip_and_order() {
        let a = ShortStr::new("husk").expect("短 id");
        let b = ShortStr::new("clay_husk").expect("短 id");
        assert_eq!(a.as_str(), "husk");
        assert!(b < a, "按字典序比较而非长度");
        assert!(ShortStr::new("a_very_long_identifier_xx").is_err());
        assert!(ShortStr::new("露").is_err());
    }

    #[test]
    fn level_order_key() {
        let key = |s: &str| LevelId::new(s).expect("短 id").order_key();
        assert!(key("1-2") < key("1-10"));
        assert_eq!(key("1-3"), (1, 3));
    }
}
