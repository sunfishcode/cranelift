//! Value affinity for register allocation.
//!
//! An SSA value's affinity is a hint used to guide the register allocator. It specifies the class
//! of allocation that is likely to cause the least amount of fixup moves in order to satisfy
//! instruction operand constraints.
//!
//! For values that want to be in registers, the affinity hint includes a register class or
//! subclass. This is just a hint, and the register allocator is allowed to pick a register from a
//! larger register class instead.

use std::fmt;
use ir::{AbiParam, ArgumentLoc};
use isa::{TargetIsa, RegInfo, RegClassIndex, OperandConstraint, ConstraintKind};

/// Preferred register allocation for an SSA value.
#[derive(Clone, Copy, Debug)]
pub enum Affinity {
    /// No affinity.
    ///
    /// This indicates a value that is not defined or used by any real instructions. It is a ghost
    /// value that won't appear in the final program.
    None,

    /// This value should be placed in a spill slot on the stack.
    Stack,

    /// This value prefers a register from the given register class.
    Reg(RegClassIndex),
}

impl Default for Affinity {
    fn default() -> Self {
        Affinity::None
    }
}

impl Affinity {
    /// Create an affinity that satisfies a single constraint.
    ///
    /// This will never create an `Affinity::None`.
    /// Use the `Default` implementation for that.
    pub fn new(constraint: &OperandConstraint) -> Affinity {
        if constraint.kind == ConstraintKind::Stack {
            Affinity::Stack
        } else {
            Affinity::Reg(constraint.regclass.into())
        }
    }

    /// Create an affinity that matches an ABI argument for `isa`.
    pub fn abi(arg: &AbiParam, isa: &TargetIsa) -> Affinity {
        match arg.location {
            ArgumentLoc::Unassigned => Affinity::None,
            ArgumentLoc::Reg(_) => Affinity::Reg(isa.regclass_for_abi_type(arg.value_type).into()),
            ArgumentLoc::Stack(_) => Affinity::Stack,
        }
    }

    /// Is this the `None` affinity?
    pub fn is_none(self) -> bool {
        match self {
            Affinity::None => true,
            _ => false,
        }
    }

    /// Is this the `Reg` affinity?
    pub fn is_reg(self) -> bool {
        match self {
            Affinity::Reg(_) => true,
            _ => false,
        }
    }

    /// Is this the `Stack` affinity?
    pub fn is_stack(self) -> bool {
        match self {
            Affinity::Stack => true,
            _ => false,
        }
    }

    /// Merge an operand constraint into this affinity.
    ///
    /// Note that this does not guarantee that the register allocator will pick a register that
    /// satisfies the constraint.
    pub fn merge(&mut self, constraint: &OperandConstraint, reg_info: &RegInfo) {
        match *self {
            Affinity::None => *self = Affinity::new(constraint),
            Affinity::Reg(rc) => {
                // If the preferred register class is a subclass of the constraint, there's no need
                // to change anything.
                if constraint.kind != ConstraintKind::Stack &&
                    !constraint.regclass.has_subclass(rc)
                {
                    // If the register classes don't overlap, `intersect` returns `None`, and we
                    // just keep our previous affinity.
                    if let Some(subclass) = constraint.regclass.intersect_index(reg_info.rc(rc)) {
                        // This constraint shrinks our preferred register class.
                        *self = Affinity::Reg(subclass);
                    }
                }
            }
            Affinity::Stack => {}
        }
    }

    /// Return an object that can display this value affinity, using the register info from the
    /// target ISA.
    pub fn display<'a, R: Into<Option<&'a RegInfo>>>(self, regs: R) -> DisplayAffinity<'a> {
        DisplayAffinity(self, regs.into())
    }
}

/// Displaying an `Affinity` correctly requires the associated `RegInfo` from the target ISA.
pub struct DisplayAffinity<'a>(Affinity, Option<&'a RegInfo>);

impl<'a> fmt::Display for DisplayAffinity<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Affinity::None => write!(f, "none"),
            Affinity::Stack => write!(f, "stack"),
            Affinity::Reg(rci) => {
                match self.1 {
                    Some(regs) => write!(f, "{}", regs.rc(rci)),
                    None => write!(f, "{}", rci),
                }
            }
        }
    }
}
