use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A long-lived account type. It does not grant access to a specific case.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Member,
    Learner,
}

impl AccountType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Learner => "learner",
        }
    }

    pub const fn can_join_cases(self) -> bool {
        matches!(self, Self::Member)
    }
}

impl fmt::Display for AccountType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for AccountType {
    type Err = InvalidRole;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "member" => Ok(Self::Member),
            "learner" => Ok(Self::Learner),
            _ => Err(InvalidRole::account_type(value)),
        }
    }
}

impl TryFrom<&str> for AccountType {
    type Error = InvalidRole;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// A verified, long-lived platform capability. Capabilities are additive: an
/// account can be both a commander and a volunteer without receiving access to
/// any case until a separate membership is granted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalCapability {
    Commander,
    Volunteer,
    Admin,
}

impl GlobalCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Commander => "commander",
            Self::Volunteer => "volunteer",
            Self::Admin => "admin",
        }
    }

    pub const fn authorizes_case_role(self, case_role: CaseRole) -> bool {
        matches!(
            (self, case_role),
            (Self::Commander, CaseRole::Commander) | (Self::Volunteer, CaseRole::Volunteer)
        )
    }
}

impl fmt::Display for GlobalCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for GlobalCapability {
    type Err = InvalidRole;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "commander" => Ok(Self::Commander),
            "volunteer" => Ok(Self::Volunteer),
            "admin" => Ok(Self::Admin),
            _ => Err(InvalidRole::global_capability(value)),
        }
    }
}

impl TryFrom<&str> for GlobalCapability {
    type Error = InvalidRole;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// A role explicitly granted for one case. Case permissions must always be
/// derived from this role plus a matching membership row.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseRole {
    Family,
    Commander,
    Volunteer,
}

impl CaseRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Family => "family",
            Self::Commander => "commander",
            Self::Volunteer => "volunteer",
        }
    }
}

impl fmt::Display for CaseRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CaseRole {
    type Err = InvalidRole;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "family" => Ok(Self::Family),
            "commander" => Ok(Self::Commander),
            "volunteer" => Ok(Self::Volunteer),
            _ => Err(InvalidRole::case(value)),
        }
    }
}

impl TryFrom<&str> for CaseRole {
    type Error = InvalidRole;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug, Eq, Error, PartialEq)]
#[error("invalid {scope} role value")]
pub struct InvalidRole {
    scope: &'static str,
}

impl InvalidRole {
    const fn account_type(_value: &str) -> Self {
        Self {
            scope: "account type",
        }
    }

    const fn global_capability(_value: &str) -> Self {
        Self {
            scope: "global capability",
        }
    }

    const fn case(_value: &str) -> Self {
        Self { scope: "case" }
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountType, CaseRole, GlobalCapability};

    #[test]
    fn role_values_are_closed_sets() {
        assert_eq!("member".parse::<AccountType>(), Ok(AccountType::Member));
        assert_eq!(
            "volunteer".parse::<GlobalCapability>(),
            Ok(GlobalCapability::Volunteer)
        );
        assert_eq!("commander".parse::<CaseRole>(), Ok(CaseRole::Commander));
        assert!("operator".parse::<GlobalCapability>().is_err());
        assert!("admin".parse::<CaseRole>().is_err());
    }

    #[test]
    fn capability_and_case_role_are_separate() {
        assert!(AccountType::Member.can_join_cases());
        assert!(!AccountType::Learner.can_join_cases());
        assert!(GlobalCapability::Commander.authorizes_case_role(CaseRole::Commander));
        assert!(!GlobalCapability::Commander.authorizes_case_role(CaseRole::Volunteer));
    }
}
