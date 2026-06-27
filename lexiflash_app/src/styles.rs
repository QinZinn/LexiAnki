pub const APP_CSS: &str = r#"
:root {
  --bg: #050505;
  --panel: rgba(255,255,255,0.06);
  --panel-2: rgba(255,255,255,0.045);
  --hair: rgba(255,255,255,0.10);
  --hair-2: rgba(255,255,255,0.07);
  --text: rgba(255,255,255,0.92);
  --muted: rgba(255,255,255,0.62);
  --faint: rgba(255,255,255,0.42);
  --accent: rgba(132, 92, 255, 0.95);
  --accent-2: rgba(0, 220, 180, 0.85);
  --danger: rgba(255, 92, 130, 0.85);
  --radius-xl: 28px;
  --radius-lg: 22px;
  --radius-md: 18px;
}

* { box-sizing: border-box; }

html, body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  color: var(--text);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", "SF Pro Display", "Geist", "Satoshi", sans-serif;
  letter-spacing: -0.01em;
  background:
    radial-gradient(900px circle at 12% 12%, rgba(132, 92, 255, 0.16), transparent 55%),
    radial-gradient(950px circle at 86% 26%, rgba(0, 220, 180, 0.10), transparent 54%),
    radial-gradient(1200px circle at 40% 110%, rgba(255, 92, 130, 0.06), transparent 55%),
    var(--bg);
}

body::before {
  content: "";
  position: fixed;
  inset: 0;
  pointer-events: none;
  background:
    repeating-linear-gradient(
      90deg,
      rgba(255,255,255,0.015) 0px,
      rgba(255,255,255,0.015) 1px,
      rgba(255,255,255,0.0) 2px,
      rgba(255,255,255,0.0) 6px
    );
  opacity: 0.42;
  mix-blend-mode: overlay;
}

.app {
  height: 100%;
  width: 100%;
  padding: 26px;
}

.frame {
  height: 100%;
  border-radius: calc(var(--radius-xl) + 6px);
  background: rgba(255,255,255,0.03);
  border: 1px solid rgba(255,255,255,0.09);
  box-shadow:
    0 18px 60px rgba(0,0,0,0.55),
    inset 0 1px 0 rgba(255,255,255,0.08);
  padding: 10px;
}

.frame_inner {
  height: 100%;
  border-radius: var(--radius-xl);
  background:
    radial-gradient(1200px circle at 30% 10%, rgba(255,255,255,0.06), transparent 52%),
    rgba(0,0,0,0.28);
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.07);
  overflow: hidden;
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 20px 14px 20px;
}

.brand {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.brand_title {
  font-size: 15px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: rgba(255,255,255,0.78);
}

.brand_subtitle {
  font-size: 28px;
  line-height: 1.08;
  letter-spacing: -0.03em;
}

.actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.pill {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  padding: 11px 14px;
  border-radius: 999px;
  background: rgba(255,255,255,0.06);
  border: 1px solid rgba(255,255,255,0.10);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.08);
  color: rgba(255,255,255,0.88);
  user-select: none;
  transition: transform 700ms cubic-bezier(0.32,0.72,0,1), background 700ms cubic-bezier(0.32,0.72,0,1), border-color 700ms cubic-bezier(0.32,0.72,0,1);
}

.pill_group {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  border-radius: 999px;
  background: rgba(255,255,255,0.035);
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.06);
}

.pill:hover {
  transform: translateY(-1px);
  background: rgba(255,255,255,0.085);
  border-color: rgba(255,255,255,0.16);
}

.pill_icon {
  width: 30px;
  height: 30px;
  border-radius: 999px;
  background: rgba(255,255,255,0.07);
  border: 1px solid rgba(255,255,255,0.10);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.08);
  transition: transform 700ms cubic-bezier(0.32,0.72,0,1);
}

.pill:hover .pill_icon {
  transform: translateX(2px) translateY(-1px) scale(1.04);
}

.content {
  height: calc(100% - 76px);
  padding: 10px 14px 16px 14px;
}

.grid {
  height: 100%;
  display: grid;
  grid-template-columns: 1.6fr 1fr;
  grid-template-rows: 1fr 1fr;
  gap: 14px;
}

.card_shell {
  background: rgba(255,255,255,0.035);
  border: 1px solid rgba(255,255,255,0.085);
  border-radius: var(--radius-xl);
  padding: 8px;
  box-shadow:
    0 16px 48px rgba(0,0,0,0.38),
    inset 0 1px 0 rgba(255,255,255,0.08);
}

.card {
  height: 100%;
  border-radius: calc(var(--radius-xl) - 8px);
  background: linear-gradient(180deg, rgba(255,255,255,0.045), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.10);
  overflow: hidden;
  transition: transform 700ms cubic-bezier(0.32,0.72,0,1), border-color 700ms cubic-bezier(0.32,0.72,0,1), background 700ms cubic-bezier(0.32,0.72,0,1);
}

.card:hover {
  transform: translateY(-2px);
  border-color: rgba(255,255,255,0.16);
  background: linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0.03));
}

.card_header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 18px 18px 12px 18px;
}

.card_title {
  font-size: 16px;
  letter-spacing: 0.01em;
}

.card_hint {
  color: var(--muted);
  font-size: 12px;
  letter-spacing: 0.10em;
  text-transform: uppercase;
}

.stats_wrap {
  padding: 0 18px 18px 18px;
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 10px;
}

.dashboard_panel_body {
  padding: 0 18px 18px 18px;
}

.stat {
  border-radius: var(--radius-lg);
  background: rgba(0,0,0,0.28);
  border: 1px solid rgba(255,255,255,0.07);
  padding: 14px 14px;
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.06);
}

.stat_value {
  font-size: 30px;
  letter-spacing: -0.04em;
  line-height: 1.0;
}

.stat_label {
  margin-top: 8px;
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.deck_list {
  padding: 0 10px 12px 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
}

.deck_row {
  padding: 12px 12px;
  border-radius: var(--radius-lg);
  background: rgba(0,0,0,0.24);
  border: 1px solid rgba(255,255,255,0.07);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.06);
  display: flex;
  align-items: center;
  justify-content: space-between;
  transition: transform 700ms cubic-bezier(0.32,0.72,0,1), border-color 700ms cubic-bezier(0.32,0.72,0,1), background 700ms cubic-bezier(0.32,0.72,0,1);
}

.deck_row:hover {
  transform: translateY(-1px);
  border-color: rgba(255,255,255,0.14);
  background: rgba(0,0,0,0.30);
}

.deck_meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.deck_title {
  font-size: 14px;
  letter-spacing: -0.01em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.deck_sub {
  color: var(--muted);
  font-size: 12px;
  display: flex;
  gap: 10px;
}

.chip {
  display: inline-flex;
  align-items: center;
  padding: 7px 10px;
  border-radius: 999px;
  background: rgba(255,255,255,0.05);
  border: 1px solid rgba(255,255,255,0.09);
  color: rgba(255,255,255,0.82);
  font-size: 12px;
  letter-spacing: 0.02em;
}

.cta_card {
  position: relative;
  padding: 18px 18px;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}

.cta_card::before {
  content: "";
  position: absolute;
  inset: -1px;
  background:
    radial-gradient(520px circle at 28% 32%, rgba(132, 92, 255, 0.24), transparent 62%),
    radial-gradient(520px circle at 80% 70%, rgba(0, 220, 180, 0.18), transparent 62%);
  opacity: 0.9;
  filter: blur(18px);
  pointer-events: none;
}

.cta_inner {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.cta_title {
  font-size: 20px;
  letter-spacing: -0.02em;
  line-height: 1.1;
}

.cta_copy {
  color: var(--muted);
  font-size: 13px;
  line-height: 1.5;
  max-width: 46ch;
}

.cta_button {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: fit-content;
  padding: 12px 16px 12px 14px;
  border-radius: 999px;
  background: rgba(255,255,255,0.10);
  border: 1px solid rgba(255,255,255,0.16);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.10);
  color: rgba(255,255,255,0.92);
  transition: transform 700ms cubic-bezier(0.32,0.72,0,1), background 700ms cubic-bezier(0.32,0.72,0,1), border-color 700ms cubic-bezier(0.32,0.72,0,1);
  user-select: none;
  appearance: none;
  cursor: pointer;
  font: inherit;
}

.cta_button:hover {
  transform: translateY(-1px);
  background: rgba(255,255,255,0.13);
  border-color: rgba(255,255,255,0.22);
}

.cta_button:active {
  transform: translateY(0px) scale(0.992);
}

.cta_trail {
  width: 30px;
  height: 30px;
  border-radius: 999px;
  background: rgba(0,0,0,0.20);
  border: 1px solid rgba(255,255,255,0.10);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: transform 700ms cubic-bezier(0.32,0.72,0,1);
}

.cta_button:hover .cta_trail {
  transform: translateX(2px) translateY(-1px) scale(1.04);
}

.pill {
  appearance: none;
  cursor: pointer;
  font: inherit;
}

.pill_active {
  background: rgba(255,255,255,0.12);
  border-color: rgba(255,255,255,0.18);
}

.create_grid {
  height: 100%;
  display: grid;
  grid-template-columns: 0.95fr 1.25fr;
  gap: 14px;
}

.create_input_shell,
.create_result_shell {
  min-height: 0;
}

.create_input_card,
.create_result_card {
  display: flex;
  flex-direction: column;
}

.create_body {
  padding: 0 18px 18px 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.eyebrow {
  font-size: 11px;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--faint);
}

.create_intro {
  color: var(--muted);
  font-size: 13px;
  line-height: 1.6;
  max-width: 46ch;
}

.field_group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field_label {
  font-size: 12px;
  letter-spacing: 0.10em;
  text-transform: uppercase;
  color: var(--muted);
}

.text_input {
  width: 100%;
  border: 1px solid rgba(255,255,255,0.10);
  background: rgba(0,0,0,0.28);
  color: var(--text);
  border-radius: 18px;
  padding: 14px 16px;
  outline: none;
  font: inherit;
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.05);
}

.text_input::placeholder {
  color: var(--faint);
}

.text_input:focus {
  border-color: rgba(132, 92, 255, 0.42);
  box-shadow:
    0 0 0 1px rgba(132, 92, 255, 0.24),
    inset 0 1px 0 rgba(255,255,255,0.05);
}

.file_pick_row {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.file_path {
  min-height: 52px;
  padding: 14px 16px;
  border-radius: 18px;
  border: 1px solid rgba(255,255,255,0.08);
  background: rgba(0,0,0,0.22);
  color: var(--muted);
  font-size: 13px;
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.action_row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.error_box {
  padding: 14px 16px;
  border-radius: 18px;
  border: 1px solid rgba(255, 92, 130, 0.24);
  background: rgba(255, 92, 130, 0.10);
  color: rgba(255,255,255,0.92);
  font-size: 13px;
  line-height: 1.5;
}

.result_meta {
  padding: 0 18px 14px 18px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  border-bottom: 1px solid rgba(255,255,255,0.07);
}

.result_title {
  font-size: 22px;
  letter-spacing: -0.03em;
  line-height: 1.08;
}

.result_subline {
  color: var(--muted);
  font-size: 12px;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.vocab_list {
  flex: 1;
  min-height: 0;
  padding: 14px 10px 10px 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
}

.vocab_row {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px 14px;
  border-radius: var(--radius-lg);
  background: rgba(0,0,0,0.24);
  border: 1px solid rgba(255,255,255,0.07);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.06);
}

.vocab_lemma {
  font-size: 16px;
  letter-spacing: -0.02em;
}

.vocab_context {
  color: var(--muted);
  font-size: 13px;
  line-height: 1.55;
}

.empty_state {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 12px;
  padding: 24px 18px 18px 18px;
}

.empty_title {
  font-size: 22px;
  letter-spacing: -0.03em;
}

.empty_copy {
  color: var(--muted);
  font-size: 13px;
  line-height: 1.6;
  max-width: 48ch;
}

.deck_empty_state {
  min-height: 240px;
  justify-content: center;
}

@media (max-width: 980px) {
  .app { padding: 18px; }
  .grid {
    grid-template-columns: 1fr;
    grid-template-rows: auto;
  }
  .create_grid {
    grid-template-columns: 1fr;
  }
}
"#;
