use insta::assert_snapshot;

use super::*;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::screen::tests::ctx;

fn frame(actions: &[Action]) -> FrameInput {
    FrameInput::new(actions.to_vec(), 0.0, vec![])
}

fn draw(screen: &PortraitViewerScreen, ctx: &Ctx) -> GlyphBuffer {
    let p = &ctx.palette;
    let mut buf = GlyphBuffer::new(
        CONSOLE_W,
        CONSOLE_H,
        Cell::new('x', p.get(UiColor::Text), p.get(UiColor::PanelBg)),
    );
    screen.draw(ctx, &mut buf);
    buf
}

#[test]
fn wrap_steps_within_range() {
    assert_eq!(wrap(0, 1, 3), 1);
    assert_eq!(wrap(2, 1, 3), 0);
    assert_eq!(wrap(0, -1, 3), 2);
    assert_eq!(wrap(1, -1, 3), 0);
    assert_eq!(wrap(0, 1, 0), 0);
    assert_eq!(wrap(0, -1, 0), 0);
}

/// The test context plus a third portrait, `test_zed` (a copy of
/// `test_lord`), so up and down lead to different characters.
fn ctx3() -> Ctx {
    let mut ctx = ctx();
    let mut zed = ctx.content.portraits["test_lord"].clone();
    zed.character = "test_zed".to_owned();
    ctx.content.portraits.insert("test_zed".to_owned(), zed);
    ctx
}

#[test]
fn keys_switch_expression_and_character() {
    use Action::{Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};
    let mut ctx = ctx3();
    let mut v = PortraitViewerScreen::new();
    assert_eq!(v.name(), "portrait_viewer");
    let mut step = |v: &mut PortraitViewerScreen, a: &[Action]| {
        assert!(matches!(v.update(&mut ctx, &frame(a)), Transition::None));
    };
    let showing = |v: &PortraitViewerScreen| {
        let ctx = ctx3();
        v.showing(&ctx).map(|(i, e)| format!("{i} {e}"))
    };
    assert_eq!(showing(&v).as_deref(), Some("test_knight neutral"));
    step(&mut v, &[CursorRight]);
    assert_eq!(showing(&v).as_deref(), Some("test_knight happy"));
    step(&mut v, &[CursorLeft, CursorLeft]);
    assert_eq!(showing(&v).as_deref(), Some("test_knight surprised"));
    step(&mut v, &[CursorRight]);
    assert_eq!(showing(&v).as_deref(), Some("test_knight neutral"));
    step(&mut v, &[CursorRight, CursorDown]);
    assert_eq!(showing(&v).as_deref(), Some("test_lord neutral"));
    step(&mut v, &[CursorRight, CursorRight, Confirm]);
    assert_eq!(showing(&v).as_deref(), Some("test_lord angry"));
    step(&mut v, &[CursorDown]);
    assert_eq!(showing(&v).as_deref(), Some("test_zed neutral"));
    step(&mut v, &[CursorDown]);
    assert_eq!(showing(&v).as_deref(), Some("test_knight neutral"));
    step(&mut v, &[CursorUp]);
    assert_eq!(showing(&v).as_deref(), Some("test_zed neutral"));
    step(&mut v, &[CursorUp]);
    assert_eq!(showing(&v).as_deref(), Some("test_lord neutral"));
}

#[test]
fn cancel_closes() {
    let mut ctx = ctx();
    let mut v = PortraitViewerScreen::new();
    let t = v.update(&mut ctx, &frame(&[Action::CursorRight, Action::Cancel]));
    assert!(matches!(t, Transition::Pop));
}

#[test]
fn no_portraits() {
    let mut ctx = ctx();
    ctx.content.portraits.clear();
    let mut v = PortraitViewerScreen::new();
    for a in [Action::CursorRight, Action::CursorLeft, Action::CursorDown] {
        v.update(&mut ctx, &frame(&[a]));
    }
    assert_eq!(v.showing(&ctx), None);
    let buf = draw(&v, &ctx);
    let row: String = (0..40)
        .filter_map(|x| buf.get(x, TOP))
        .map(|c| c.glyph)
        .collect();
    assert!(
        row.starts_with(" No portraits in assets/portraits/"),
        "{row}"
    );
}

#[test]
fn help_names_the_keys() {
    let ctx = ctx();
    assert_eq!(
        PortraitViewerScreen::help(&ctx),
        "Left/Right expression · Down/Up character · d back"
    );
}

#[test]
fn viewer_snapshot() {
    let mut ctx = ctx();
    let mut v = PortraitViewerScreen::new();
    v.update(&mut ctx, &frame(&[Action::CursorDown, Action::CursorRight]));
    assert_snapshot!(draw(&v, &ctx).to_snapshot(&ctx.palette));
}
