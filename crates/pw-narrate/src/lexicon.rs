//! The language layer (X, Y in the media/social brief).
//!
//! Meaning is decided upstream as semantic structures — a post's stance and
//! references, a story's claim type and angle, a question's topic. This
//! module only chooses *words*, for a locale, a register and a voice. Adding
//! a language means adding a pack; nothing upstream changes.
//!
//! Voices come from who is speaking (an account's age band, style and
//! temperament; an outlet's register), never from nationality: no pack may
//! caricature where someone is from.

use pw_world::LifeEventKind;

use crate::pick;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Locale {
    En,
}

/// How formal or loud the language is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Register {
    /// Broadsheet, official statements.
    Formal,
    /// Plain reporting.
    Neutral,
    /// Everyday supporter talk.
    Casual,
    /// Headline-shouting.
    Tabloid,
    /// The noisiest end of the support.
    Terrace,
    /// Numbers people.
    Analytical,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AgeBand {
    Young,
    Middle,
    Older,
}

/// Everything word choice depends on.
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    pub locale: Locale,
    pub register: Register,
    pub age: AgeBand,
    /// Uses emoji (only in casual registers).
    pub emoji: bool,
    /// 0–100: likes to joke.
    pub humour: u8,
    /// 0–100: hedges claims ("I think", "apparently").
    pub hedging: u8,
}

impl Voice {
    pub const fn neutral() -> Self {
        Voice { locale: Locale::En, register: Register::Neutral, age: AgeBand::Middle, emoji: false, humour: 20, hedging: 40 }
    }
}

/// Word slots a renderer can ask for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Slot {
    PraiseAdj,
    PraiseStrong,
    CriticAdj,
    CriticStrong,
    Intensifier,
    Hedge,
    Celebrate,
    Lament,
    Mock,
    Concede,
    DoubleDown,
    Worry,
    RumourWeak,
    RumourStrong,
    Denial,
    Nostalgia,
    Sarcasm,
    Shrug,
    Anger,
    Loyal,
    Patience,
    Impatience,
    Question,
    Positive,
    Negative,
}

type Words = &'static [&'static str];

fn en(register: Register, age: AgeBand, slot: Slot) -> Words {
    use Register::*;
    use Slot::*;
    let young = age == AgeBand::Young;
    match (slot, register) {
        (PraiseAdj, Formal | Neutral | Analytical) => &["excellent", "impressive", "outstanding", "assured", "influential"],
        (PraiseAdj, Tabloid) => &["sensational", "stunning", "magnificent", "unstoppable"],
        (PraiseAdj, _) if young => &["unreal", "different class", "cold", "elite", "levels above"],
        (PraiseAdj, _) => &["brilliant", "superb", "class", "top drawer", "quality"],
        (PraiseStrong, Formal | Neutral) => &["exceptional", "a performance of real authority", "decisive"],
        (PraiseStrong, Analytical) => &["off the charts", "elite by every measure", "top percentile"],
        (PraiseStrong, Tabloid) => &["a hero", "a sensation", "pure genius"],
        (PraiseStrong, _) if young => &["actually him", "the GOAT of this club", "on another planet"],
        (PraiseStrong, _) => &["a legend in the making", "worth every penny", "magnificent"],
        (CriticAdj, Formal | Neutral) => &["poor", "off the pace", "below their usual level", "unconvincing"],
        (CriticAdj, Analytical) => &["underperforming the numbers", "below expected output", "in the bottom quartile"],
        (CriticAdj, Tabloid) => &["woeful", "shambolic", "a flop"],
        (CriticAdj, _) if young => &["mid", "finished", "washed", "not it"],
        (CriticAdj, _) => &["poor", "shocking", "not good enough", "dreadful"],
        (CriticStrong, Formal | Neutral | Analytical) => &["deeply concerning", "a serious problem", "hard to defend"],
        (CriticStrong, Tabloid) => &["a DISASTER", "a total embarrassment", "a nightmare"],
        (CriticStrong, _) => &["an absolute disgrace", "the worst I've seen", "embarrassing"],
        (Intensifier, Formal | Neutral | Analytical) => &["very", "highly", "remarkably"],
        (Intensifier, _) if young => &["so", "genuinely", "literally"],
        (Intensifier, _) => &["really", "properly", "absolutely"],
        (Hedge, Formal | Neutral) => &["it appears", "reportedly", "it is understood"],
        (Hedge, Analytical) => &["the data suggests", "on current numbers", "small sample, but"],
        (Hedge, _) => &["apparently", "I'm hearing", "rumour is", "not confirmed but"],
        (Celebrate, Formal | Neutral | Analytical) => &["a significant result", "a valuable win", "a welcome victory"],
        (Celebrate, Tabloid) => &["GET IN", "WHAT A NIGHT", "SCENES"],
        (Celebrate, _) if young => &["SCENES", "let's gooo", "we move", "what a day"],
        (Celebrate, _) => &["get in", "what a result", "magnificent", "love this team"],
        (Lament, Formal | Neutral | Analytical) => &["a disappointing result", "a setback", "a costly afternoon"],
        (Lament, _) if young => &["pain", "can't watch this anymore", "why do we do this to ourselves"],
        (Lament, _) => &["gutted", "sick of this", "same old story", "painful to watch"],
        (Mock, Formal | Neutral | Analytical) => &["a chastening day for the visitors", "a difficult afternoon for them"],
        (Mock, _) if young => &["enjoy that one", "sit down", "who are you again", "lol"],
        (Mock, _) => &["enjoy the trip home", "silence", "not so loud now", "thanks for coming"],
        (Concede, _) if young => &["ok fair play, I was wrong", "I'll hold my hands up", "I've been cooked"],
        (Concede, _) => &["fair enough, proved me wrong", "hands up, I was wrong about this one", "credit where it's due"],
        (DoubleDown, _) => &["one good game doesn't change anything", "still not convinced", "let's see it every week", "flat-track stuff"],
        (Worry, Formal | Neutral | Analytical) => &["there is some concern about", "questions remain over"],
        (Worry, _) => &["worried about", "not sure about", "nervous about"],
        (RumourWeak, Formal | Neutral) => &["are monitoring", "are aware of", "have made preliminary enquiries about"],
        (RumourWeak, Tabloid) => &["are eyeing", "are keeping tabs on"],
        (RumourWeak, _) => &["are apparently watching", "have been looking at", "are sniffing around"],
        (RumourStrong, Formal | Neutral) => &["are preparing a move for", "are expected to bid for"],
        (RumourStrong, Tabloid) => &["PLOT SHOCK SWOOP FOR", "are READY TO RAID for", "LINE UP BUMPER BID FOR"],
        (RumourStrong, _) => &["are going all in for", "want this one badly —", "are about to bid for"],
        (Denial, _) => &["no truth in it", "nothing to it", "complete fiction", "not happening"],
        (Nostalgia, _) => &["reminds me of", "not seen anything like it since", "proper throwback to"],
        (Sarcasm, _) if young => &["great, love that for us", "incredible scenes (not)", "ok sure"],
        (Sarcasm, _) => &["marvellous", "brilliant, just brilliant", "well that went well"],
        (Shrug, _) => &["whatever", "we move on", "it is what it is", "next game"],
        (Anger, Tabloid) => &["FURY", "OUTRAGE", "FUMING"],
        (Anger, _) if young => &["actually fuming", "raging", "I'm done"],
        (Anger, _) => &["furious", "livid", "absolutely raging"],
        (Loyal, _) => &["one of our own", "a proper club servant", "gets this club"],
        (Patience, _) => &["give him time", "be patient", "early days"],
        (Impatience, _) => &["time's up", "seen enough", "it's not working"],
        (Question, Formal | Neutral | Analytical) => &["What next for", "Where now for", "Is it time for"],
        (Question, _) => &["what's going on with", "anyone else worried about", "is it just me or"],
        (Positive, _) => &["good", "positive", "encouraging"],
        (Negative, _) => &["bad", "worrying", "grim"],
    }
}

/// The words available for a slot in a voice.
pub fn words(v: &Voice, slot: Slot) -> Words {
    match v.locale {
        Locale::En => en(v.register, v.age, slot),
    }
}

/// One word or phrase for a slot, stable for a key.
pub fn word(v: &Voice, slot: Slot, key: u64) -> &'static str {
    pick(key ^ slot as u64 * 0x9e37, words(v, slot))
}

/// Emoji for a sentiment, only where the voice uses them.
pub fn emoji(v: &Voice, positive: bool, key: u64) -> &'static str {
    if !v.emoji {
        return "";
    }
    if positive { pick(key, &[" 🔥", " 👏", " 💪", " 🙌", ""]) } else { pick(key, &[" 😤", " 🤦", " 😬", " 💀", ""]) }
}

/// Shout it (tabloid headlines, the loudest accounts).
pub fn loud(v: &Voice, s: &str) -> String {
    if matches!(v.register, Register::Tabloid) { s.to_uppercase() } else { s.to_string() }
}

/// Plain words for private life events (used when information about them travels).
pub fn life_event(k: LifeEventKind) -> &'static str {
    match k {
        LifeEventKind::StartedDating { .. } => "a new relationship",
        LifeEventKind::MovedIn { .. } => "moving in with a partner",
        LifeEventKind::Married { .. } => "a wedding",
        LifeEventKind::Separated { .. } => "a separation",
        LifeEventKind::ChildBorn => "a new baby",
        LifeEventKind::ParentUnwell => "a parent's illness",
        LifeEventKind::ParentRecovered => "a parent's recovery",
        LifeEventKind::Bereavement => "a bereavement",
        LifeEventKind::Relocated { .. } => "a move abroad",
        LifeEventKind::PartnerJoinedMove { .. } => "a partner joining a move",
        LifeEventKind::PartnerStayedBehind { .. } => "a partner staying behind",
        LifeEventKind::FinancialTrouble => "money trouble",
        LifeEventKind::Graduated => "graduating",
    }
}
