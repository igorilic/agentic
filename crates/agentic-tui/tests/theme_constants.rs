use agentic_tui::theme;
use ratatui::style::Color;

#[test]
fn bg_matches_spec() {
    assert_eq!(theme::BG, Color::Rgb(0x0d, 0x0e, 0x10));
}

#[test]
fn fg_matches_spec() {
    assert_eq!(theme::FG, Color::Rgb(0xe6, 0xe6, 0xe6));
}

#[test]
fn dim_matches_spec() {
    assert_eq!(theme::DIM, Color::Rgb(0x7d, 0x7d, 0x8a));
}

#[test]
fn border_matches_spec() {
    assert_eq!(theme::BORDER, Color::Rgb(0x2a, 0x2b, 0x30));
}

#[test]
fn accent_matches_spec() {
    assert_eq!(theme::ACCENT, Color::Rgb(0x5e, 0xea, 0xd4));
}

#[test]
fn blue_matches_spec() {
    assert_eq!(theme::BLUE, Color::Rgb(0x7d, 0xd3, 0xfc));
}

#[test]
fn yellow_matches_spec() {
    assert_eq!(theme::YELLOW, Color::Rgb(0xfd, 0xe6, 0x8a));
}

#[test]
fn green_matches_spec() {
    assert_eq!(theme::GREEN, Color::Rgb(0xa7, 0xf3, 0xd0));
}

#[test]
fn red_matches_spec() {
    assert_eq!(theme::RED, Color::Rgb(0xfc, 0xa5, 0xa5));
}

#[test]
fn purple_matches_spec() {
    assert_eq!(theme::PURPLE, Color::Rgb(0xc4, 0xb5, 0xfd));
}

#[test]
fn header_bg_matches_spec() {
    assert_eq!(theme::HEADER_BG, Color::Rgb(0x16, 0x17, 0x1b));
}
