//! The battle's one-time tips (0406): which triggers have fired, the tips
//! waiting to be shown, and which the player has already seen.

use std::collections::VecDeque;

use trpg_content::TipTrigger;

use crate::screen::Ctx;
use crate::tips::TipsSeen;

/// The tips of one battle screen. A trigger that fires is queued if its tip
/// has not been seen (and is recorded as seen at once, so it shows at most
/// once per profile); the screen shows the queue front when the moment is
/// right and drops it when the player dismisses it.
#[derive(Debug, Clone, Default)]
pub struct TipState {
    /// Loaded from storage on first use.
    seen: Option<TipsSeen>,
    /// Triggers that fired since the last [`absorb`](Self::absorb).
    fired: Vec<TipTrigger>,
    /// Tips waiting to be shown, first in front.
    queue: VecDeque<TipTrigger>,
}

impl TipState {
    /// Notes that `trigger` happened.
    pub fn fire(&mut self, trigger: TipTrigger) {
        if !self.fired.contains(&trigger) {
            self.fired.push(trigger);
        }
    }

    /// Queues the fired triggers whose tip exists and hasn't been seen, and
    /// saves them as seen. With tips off, they are just forgotten.
    pub fn absorb(&mut self, ctx: &mut Ctx) {
        let fired = std::mem::take(&mut self.fired);
        if !ctx.tips_enabled {
            return;
        }
        let seen = self
            .seen
            .get_or_insert_with(|| TipsSeen::load(&*ctx.storage));
        for trigger in fired {
            let Some(tip) = ctx.content.tips.for_trigger(trigger) else {
                continue;
            };
            if !seen.contains(&tip.id) {
                seen.mark(&tip.id, &mut *ctx.storage);
                self.queue.push_back(trigger);
            }
        }
    }

    /// The queued tips' triggers, first to show first.
    pub fn queued(&self) -> impl Iterator<Item = TipTrigger> + '_ {
        self.queue.iter().copied()
    }

    /// Drops `trigger`'s tip from the queue.
    pub fn dismiss(&mut self, trigger: TipTrigger) {
        self.queue.retain(|&t| t != trigger);
    }
}
