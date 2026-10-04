//! Validated name newtypes.
//!
//! A name is `^[a-z0-9]+(-[a-z0-9]+)*$`, 1–64 chars (per the Agent Skills
//! standard). Validation is the *only* constructor, so a value of these types is
//! always safe to join onto a filesystem path — an unvalidated `../../etc` can
//! never become a [`SkillFullName`].

use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Maximum length of any skill/bundle name, in characters.
pub const MAX_NAME_LEN: usize = 64;

static NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").expect("static regex is valid"));

/// Validate a raw name against the shared contract, tagging errors with `kind`.
fn validate(kind: &'static str, value: &str) -> Result<()> {
    let len = value.chars().count();
    if !(1..=MAX_NAME_LEN).contains(&len) {
        return Err(Error::InvalidName {
            kind,
            value: value.to_string(),
            reason: "must be 1-64 characters",
        });
    }
    if !NAME_RE.is_match(value) {
        return Err(Error::InvalidName {
            kind,
            value: value.to_string(),
            reason: "lowercase letters, digits, single interior hyphens only",
        });
    }
    Ok(())
}

/// Generate a validated name newtype plus its `serde`/`Display`/`AsRef` impls.
///
/// The `try_from`/`into` serde attributes route (de)serialization through the
/// validated constructor, so the invariant holds across TOML/JSON boundaries too.
macro_rules! name_newtype {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// Parse and validate a raw name.
            pub fn parse(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                validate($kind, &value)?;
                Ok(Self(value))
            }

            /// The underlying string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = Error;
            fn try_from(value: String) -> Result<Self> {
                Self::parse(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }
    };
}

name_newtype!(
    /// The name of a bundle (`bundles/<name>.toml`).
    BundleName,
    "bundle"
);

name_newtype!(
    /// A skill's full, bundle-prefixed name (`lgit-pages-html`).
    SkillFullName,
    "skill"
);

name_newtype!(
    /// A skill's short name with the bundle prefix stripped (`pages-html`).
    SkillShortName,
    "skill"
);

impl SkillFullName {
    /// Strip the `<bundle>-` prefix, returning the short name.
    ///
    /// Fails if the full name is not prefixed by `bundle` (a `-` separator is
    /// required), or if the remainder is not itself a valid name.
    pub fn require_prefix(&self, bundle: &BundleName) -> Result<SkillShortName> {
        let prefix = format!("{}-", bundle.as_str());
        match self.0.strip_prefix(&prefix) {
            Some(short) => SkillShortName::parse(short),
            None => Err(Error::BadPrefix {
                bundle: bundle.as_str().to_string(),
                skill: self.0.clone(),
            }),
        }
    }
}
