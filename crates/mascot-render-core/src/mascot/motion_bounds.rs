use crate::{MascotConfig, SquashBounceAnimationConfig};

impl MascotConfig {
    /// Reserve enough space for both the click reaction and editable playback motion.
    pub fn squash_motion_bounds(&self) -> SquashBounceAnimationConfig {
        SquashBounceAnimationConfig {
            amplitude_px: self
                .squash_bounce
                .amplitude_px
                .max(self.musical_squash_bounce.amplitude_px),
            squash_amount: self
                .squash_bounce
                .squash_amount
                .max(self.musical_squash_bounce.squash_amount),
            stretch_amount: self
                .squash_bounce
                .stretch_amount
                .max(self.musical_squash_bounce.stretch_amount),
            ..self.squash_bounce
        }
    }
}
