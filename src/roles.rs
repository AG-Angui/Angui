use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A platform-wide account role. It determines platform entry points and
/// eligibility, but never grants access to a specific case by itself.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalRole {
    Family,
    Commander,
    Volunteer,
    Learner,
    Admin,
}

impl GlobalRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Family => "family",
            Self::Commander => "commander",
            Self::Volunteer => "volunteer",
            Self::Learner => "learner",
            Self::Admin => "admin",
        }
    }

    pub const fn initial_case_role(self) -> Option<CaseRole> {
        match self {
            Self::Family => Some(CaseRole::Family),
            Self::Commander => Some(CaseRole::Commander),
            Self::Volunteer | Self::Learner | Self::Admin => None,
        }
    }

    /// Only operational accounts may receive a role in a case. Platform
    /// administrators and learning-only accounts remain outside case data.
    pub const fn can_receive_case_role(self) -> bool {
        matches!(self, Self::Family | Self::Commander | Self::Volunteer)
    }
}

impl fmt::Display for GlobalRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for GlobalRole {
    type Err = InvalidRole;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "family" => Ok(Self::Family),
            "commander" => Ok(Self::Commander),
            "volunteer" => Ok(Self::Volunteer),
            "learner" => Ok(Self::Learner),
            "admin" => Ok(Self::Admin),
            _ => Err(InvalidRole::global(value)),
        }
    }
}

impl TryFrom<&str> for GlobalRole {
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
    const fn global(_value: &str) -> Self {
        Self { scope: "global" }
    }

    const fn case(_value: &str) -> Self {
        Self { scope: "case" }
    }
}

#[cfg(test)]
mod tests {
    use super::{CaseRole, GlobalRole};

    #[test]
    fn role_values_are_closed_sets() {
        assert_eq!("family".parse::<GlobalRole>(), Ok(GlobalRole::Family));
        assert_eq!("commander".parse::<CaseRole>(), Ok(CaseRole::Commander));
        assert!("operator".parse::<GlobalRole>().is_err());
        assert!("admin".parse::<CaseRole>().is_err());
    }

    #[test]
    fn only_operational_accounts_can_receive_case_roles() {
        assert!(GlobalRole::Family.can_receive_case_role());
        assert!(GlobalRole::Commander.can_receive_case_role());
        assert!(GlobalRole::Volunteer.can_receive_case_role());
        assert!(!GlobalRole::Learner.can_receive_case_role());
        assert!(!GlobalRole::Admin.can_receive_case_role());
    }
}
