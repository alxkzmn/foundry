use p3_field::extension::{BinomialExtensionField, QuinticTrinomialExtensionField};
use p3_koala_bear::KoalaBear;

pub type F = KoalaBear;
pub type QuinticTrinomialExtension = QuinticTrinomialExtensionField<F>;
pub type OcticBinExtension = BinomialExtensionField<F, 8>;
