//! The development-only door to the world. Normal pages never come through here: they read the world only through `Ctx`, which shows
//! exact truth in one view alone (the omniscient debug view, `Ctx::observer`). This module is for tests of the information firewall, which
//! change hidden truth and check that what a non-omniscient viewer sees does not move (locked design 8.8, 8.10).

use pw_world::World;
use serde_json::{Value, json};

use crate::Api;
use crate::ctx::Ctx;

impl Api {
    /// Change the open world directly, or `None` if no world is open. **Debug and test use only.**
    #[doc(hidden)]
    pub fn debug_mutate_world<R>(&self, f: impl FnOnce(&mut World) -> R) -> Option<R> {
        let mut g = self.lock();
        let s = g.as_mut()?;
        let r = f(&mut s.game.sim.world);
        s.revision += 1;
        Some(r)
    }

    /// What the view-type layer holds for this person as the current viewer, as JSON: `{player, ability}` (`VisiblePlayer`,
    /// `VisibleAbility`), or `None` if no world is open or he is not a player. **Debug and test use only.** The firewall tests use it to
    /// show that the type itself cannot carry hidden truth to a public or inhabited viewer.
    #[doc(hidden)]
    pub fn debug_visible_player(&self, person: u32) -> Option<Value> {
        let g = self.lock();
        let s = g.as_ref()?;
        let c = Ctx::new(s);
        let p = c.w.people.get(pw_core::PersonId(person))?.player.get()?;
        Some(json!({"player": c.visible_player(p), "ability": c.visible_ability(p)}))
    }
}
