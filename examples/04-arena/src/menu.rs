//! Start-screen state: pages, layout, hover animation and the sensitivity
//! slider. Pure math over HUD space (pixels, y up) — no GPU types — so the
//! whole flow unit-tests. `main.rs` renders items from the same [`layout`]
//! the hit tests use, which keeps what you see and what you click aligned
//! by construction.

use glam::{Vec2, vec2};
use vex_core::font;

pub const TITLE: &str = "VECTOR3D";
pub const SUBTITLE: &str = "ARENA";

/// Base label height; hovered items render up to [`HOVER_GROW`] larger.
pub const BUTTON_PX: f32 = 30.0;
/// Fractional size growth at full hover.
pub const HOVER_GROW: f32 = 0.16;
const ROW_GAP: f32 = 62.0;
/// Hit-box padding around a label. The box ignores hover growth so the
/// hovered item can't flicker at its own expanding edge.
const PAD: f32 = 14.0;
/// How fast hover brightness/growth chases the cursor (per second).
const HOVER_RATE: f32 = 14.0;

pub const SLIDER_WIDTH: f32 = 380.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Main,
    Options,
}

/// What the caller must act on this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Play,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Play,
    Options,
    Quit,
    Back,
    Slider,
}

/// One interactive element, positioned in HUD space (pixels, y up).
pub struct Item {
    pub kind: ItemKind,
    pub label: &'static str,
    /// Bottom-left corner of the label at `px` height (font origin).
    pub origin: Vec2,
    pub px: f32,
    /// Hit box: (min, max). Stable under hover growth.
    pub min: Vec2,
    pub max: Vec2,
}

impl Item {
    fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

fn button(kind: ItemKind, label: &'static str, center_y: f32, viewport: Vec2) -> Item {
    let width = font::text_width(label, BUTTON_PX);
    let origin = vec2((viewport.x - width) * 0.5, center_y - BUTTON_PX * 0.5);
    Item {
        kind,
        label,
        origin,
        px: BUTTON_PX,
        min: origin - Vec2::splat(PAD),
        max: origin + vec2(width, BUTTON_PX) + Vec2::splat(PAD),
    }
}

/// The current page's interactive items, top to bottom.
pub fn layout(page: Page, viewport: Vec2) -> Vec<Item> {
    let top = viewport.y * 0.52;
    match page {
        Page::Main => {
            let mut items = vec![
                button(ItemKind::Play, "PLAY", top, viewport),
                button(ItemKind::Options, "OPTIONS", top - ROW_GAP, viewport),
            ];
            // No window to close in a browser tab.
            #[cfg(not(target_arch = "wasm32"))]
            items.push(button(ItemKind::Quit, "QUIT", top - 2.0 * ROW_GAP, viewport));
            items
        }
        Page::Options => {
            let slider_center = vec2(viewport.x * 0.5, top - ROW_GAP);
            let half = vec2(SLIDER_WIDTH * 0.5 + PAD, 24.0);
            let mut slider = button(ItemKind::Slider, "SENSITIVITY", top, viewport);
            slider.min = slider_center - half;
            slider.max = slider_center + half;
            vec![
                slider,
                button(ItemKind::Back, "BACK", top - 2.2 * ROW_GAP, viewport),
            ]
        }
    }
}

/// Track endpoints (left, right) for the slider item, in HUD space.
pub fn slider_track(item: &Item) -> (Vec2, Vec2) {
    let center = (item.min + item.max) * 0.5;
    (
        vec2(center.x - SLIDER_WIDTH * 0.5, center.y),
        vec2(center.x + SLIDER_WIDTH * 0.5, center.y),
    )
}

/// Piecewise-linear sensitivity multiplier: slider 0 → 0.25×, the center
/// detent → 1× (the authored default), 1 → 3×.
pub fn sensitivity_scale(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v < 0.5 {
        0.25 + (1.0 - 0.25) * (v / 0.5)
    } else {
        1.0 + (3.0 - 1.0) * ((v - 0.5) / 0.5)
    }
}

pub struct Menu {
    pub page: Page,
    /// Slider position 0..1; 0.5 is the authored default sensitivity.
    pub sensitivity: f32,
    /// Animated hover amount per item index of the current page.
    pub hover: [f32; 4],
    dragging: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            page: Page::Main,
            sensitivity: 0.5,
            hover: [0.0; 4],
            dragging: false,
        }
    }

    /// Advance one frame: `cursor` in HUD space (y up), `click` on the
    /// press edge, `held` while the button stays down (slider drags).
    pub fn update(
        &mut self,
        dt: f32,
        viewport: Vec2,
        cursor: Vec2,
        click: bool,
        held: bool,
    ) -> Action {
        if !held {
            self.dragging = false;
        }
        let items = layout(self.page, viewport);
        let mut action = Action::None;
        let mut hovered = usize::MAX;
        for (i, item) in items.iter().enumerate() {
            let inside = item.contains(cursor);
            if inside {
                hovered = i;
            }
            if item.kind == ItemKind::Slider {
                if click && inside {
                    self.dragging = true;
                }
                if self.dragging {
                    let (left, right) = slider_track(item);
                    self.sensitivity = ((cursor.x - left.x) / (right.x - left.x)).clamp(0.0, 1.0);
                }
            } else if click && inside {
                action = self.activate(item.kind);
            }
        }
        // Hover eases toward its target so items swell and brighten
        // instead of popping.
        let ease = 1.0 - (-HOVER_RATE * dt).exp();
        for (i, h) in self.hover.iter_mut().enumerate() {
            let target = if i == hovered { 1.0 } else { 0.0 };
            *h += (target - *h) * ease;
        }
        action
    }

    fn activate(&mut self, kind: ItemKind) -> Action {
        match kind {
            ItemKind::Play => Action::Play,
            ItemKind::Quit => Action::Quit,
            ItemKind::Options => {
                self.page = Page::Options;
                self.hover = [0.0; 4];
                Action::None
            }
            ItemKind::Back => {
                self.page = Page::Main;
                self.hover = [0.0; 4];
                Action::None
            }
            ItemKind::Slider => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Vec2 = vec2(1280.0, 720.0);

    fn center_of(page: Page, kind: ItemKind) -> Vec2 {
        let items = layout(page, VIEW);
        let item = items.iter().find(|i| i.kind == kind).unwrap();
        (item.min + item.max) * 0.5
    }

    #[test]
    fn click_play_starts_the_game() {
        let mut menu = Menu::new();
        let at = center_of(Page::Main, ItemKind::Play);
        assert_eq!(menu.update(0.016, VIEW, at, true, true), Action::Play);
    }

    #[test]
    fn options_and_back_flip_pages() {
        let mut menu = Menu::new();
        let at = center_of(Page::Main, ItemKind::Options);
        assert_eq!(menu.update(0.016, VIEW, at, true, true), Action::None);
        assert_eq!(menu.page, Page::Options);
        let back = center_of(Page::Options, ItemKind::Back);
        menu.update(0.016, VIEW, back, true, true);
        assert_eq!(menu.page, Page::Main);
    }

    #[test]
    fn quit_quits() {
        let mut menu = Menu::new();
        let at = center_of(Page::Main, ItemKind::Quit);
        assert_eq!(menu.update(0.016, VIEW, at, true, true), Action::Quit);
    }

    #[test]
    fn hover_eases_up_on_the_pointed_item_only() {
        let mut menu = Menu::new();
        let at = center_of(Page::Main, ItemKind::Play);
        menu.update(0.016, VIEW, at, false, false);
        let first = menu.hover[0];
        assert!(first > 0.0 && first < 1.0, "eases, not pops: {first}");
        menu.update(0.016, VIEW, at, false, false);
        assert!(menu.hover[0] > first, "keeps growing while hovered");
        assert_eq!(menu.hover[1], 0.0, "unhovered items stay idle");
        // Pointing at nothing decays it again.
        menu.update(0.5, VIEW, vec2(5.0, 5.0), false, false);
        assert!(menu.hover[0] < first);
    }

    #[test]
    fn slider_drags_clamps_and_releases() {
        let mut menu = Menu::new();
        menu.page = Page::Options;
        let items = layout(Page::Options, VIEW);
        let slider = items.iter().find(|i| i.kind == ItemKind::Slider).unwrap();
        let (left, right) = slider_track(slider);
        let mid = (left + right) * 0.5;

        // Press on the track, drag off the right end: clamps to 1.
        menu.update(0.016, VIEW, mid, true, true);
        menu.update(0.016, VIEW, right + vec2(300.0, 0.0), false, true);
        assert_eq!(menu.sensitivity, 1.0);

        // Release, then move: value stays put.
        menu.update(0.016, VIEW, mid, false, false);
        assert_eq!(menu.sensitivity, 1.0, "no drag after release");

        // Fresh press snaps to the cursor.
        menu.update(0.016, VIEW, left, true, true);
        assert_eq!(menu.sensitivity, 0.0);
    }

    #[test]
    fn sensitivity_scale_maps_the_detents() {
        assert!((sensitivity_scale(0.0) - 0.25).abs() < 1e-6);
        assert!((sensitivity_scale(0.5) - 1.0).abs() < 1e-6);
        assert!((sensitivity_scale(1.0) - 3.0).abs() < 1e-6);
        assert!(sensitivity_scale(-1.0) >= 0.25 && sensitivity_scale(2.0) <= 3.0);
    }
}
