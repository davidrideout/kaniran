//! Port of `ichiran/dict:*hint-map*` (`dict-split.lisp:850`).
//!
//! Hashtable mapping JMdict seq → hint function, registered
//! upstream by the [`defhint`](`dict-split.lisp:852`) macro and its
//! two callers [`def-simple-hint`](`dict-split.lisp:860`) and
//! [`def-easy-hint`](`dict-split.lisp:916`). Each callsite expands
//! into one or more `(setf (gethash ,seq *hint-map*) ,fn)` forms.
//! The live image has 659 entries after `dict-split.lisp` loads
//! (verified via REPL: `(hash-table-count *hint-map*) => 659`).
//!
//! The Rust transliteration collapses the runtime hashtable into a
//! static dispatcher driven by two tables:
//!
//! - [`EASY_HINTS`] — 431 rows, one per `def-easy-hint` callsite.
//!   Each row carries the literal kanji-split string; the shared
//!   body lives in [`crate::dict::kani::run_easy_hint`].
//! - 17 inline `match`-arm groups, one per `def-simple-hint`
//!   callsite — bodies vary per group (different positional
//!   computations, different `:test`/`let*`-binding shapes).
//!
//! ## Divergence from CONVENTIONS §1
//!
//! CONVENTIONS §1 (one Lisp symbol per Rust file) is intentionally
//! relaxed here, mirroring the [`super::_star_split_map_star_`]
//! precedent. The 17 `def-simple-hint` and 431 `def-easy-hint`
//! callsites would otherwise need 448 separate files. Putting them
//! here keeps the data and dispatcher together. The macros
//! themselves (`defhint`, `def-simple-hint`, `def-easy-hint`) are
//! marked `skip` with reason pointing at this file per §4.6 case (a).
//!
//! ## Order semantics
//!
//! Upstream `(setf (gethash ...) ...)` repeats: later calls override
//! earlier ones for the same seq. In `dict-split.lisp`, all 17
//! `def-simple-hint` forms (lines 1014-1382) precede all 431
//! `def-easy-hint` forms (lines 1389-1859), so when a seq is
//! registered by both, the easy-hint body wins. This dispatcher
//! mirrors that: it checks [`EASY_HINTS`] before the simple-hint
//! match arms.
//!
//! ## Hint-state contract
//!
//! Called from [`super::hint::get_hint`], which is itself called
//! from the `simple-text :around` method on `get-kana` under a ctx
//! rebound via
//! [`crate::conn::kani_context::KaniranContext::with_disable_hints`]`(true)`.
//! Hint bodies in turn call [`crate::dict::best_text::get_kana`] and
//! [`crate::dict::best_text::true_kana`]; the inner `:around` reads
//! `ctx.disable_hints = true` and skips the hint branch (matches the
//! upstream `*disable-hints*` rebind at `dict.lisp:82`).

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::conn::kani_context::KaniranContext;
use crate::dict::kani::{
    ends_with_char, finish_simple_hint, run_easy_hint, safe_hint, search_chars,
    true_kana_and_len, EasyHint,
};
use crate::dict::kani::KaniHintKind;
use crate::dict::kani::KaniWordDispatchEnum;

/// Static dispatch table for the 431 `def-easy-hint` callsites at
/// `dict-split.lisp:1389-1859`. Source order is preserved (the
/// upstream `(push ,seq *easy-hints-seqs*)` orders the seqs list
/// differently — see [`super::_star_easy_hints_seqs_star_`] for
/// that semantic — but the dispatcher reads this as a set, so the
/// order doesn't affect lookup semantics).
pub static EASY_HINTS: &[EasyHint] = &[
    EasyHint { seq: 1238480, kanji_split: "郷 に 入って は 郷 に 従え" },
    EasyHint { seq: 1338260, kanji_split: "出る 釘 は 打たれる" },
    EasyHint { seq: 1471680, kanji_split: "馬鹿 は 死ななきゃ 治らない" },
    EasyHint { seq: 1566340, kanji_split: "屁 と 火事 は もと から 騒ぐ" },
    EasyHint { seq: 1638390, kanji_split: "用心する に 如く は ない" },
    EasyHint { seq: 1985430, kanji_split: "出る 杭 は 打たれる" },
    EasyHint { seq: 2078950, kanji_split: "背 に 腹 は かえられない" },
    EasyHint { seq: 2083430, kanji_split: "瓜 の 蔓 に 茄子 は ならぬ" },
    EasyHint { seq: 2093380, kanji_split: "敵 は 本能寺 に あり" },
    EasyHint { seq: 2101500, kanji_split: "無い 袖 は 振れぬ" },
    EasyHint { seq: 2109530, kanji_split: "継続 は 力 なり" },
    EasyHint { seq: 2111700, kanji_split: "事実 は 小説 より 奇 なり" },
    EasyHint { seq: 2111710, kanji_split: "事実 は 小説 よりも 奇 なり" },
    EasyHint { seq: 2113730, kanji_split: "縁 は 異なもの 味なもの" },
    EasyHint { seq: 2113740, kanji_split: "縁は異なもの" },
    EasyHint { seq: 2125770, kanji_split: "その手 は 食わない" },
    EasyHint { seq: 2130410, kanji_split: "すまじき もの は 宮仕え" },
    EasyHint { seq: 2140350, kanji_split: "時 は 金 なり" },
    EasyHint { seq: 2140990, kanji_split: "青 は 藍 より 出でて 藍 より 青し" },
    EasyHint { seq: 2152710, kanji_split: "渡る 世間 に 鬼 は ない" },
    EasyHint { seq: 2152850, kanji_split: "病 は 気 から" },
    EasyHint { seq: 2152960, kanji_split: "甲 の 薬 は 乙 の 毒" },
    EasyHint { seq: 2159970, kanji_split: "好奇心 は 猫 をも 殺す" },
    EasyHint { seq: 2159990, kanji_split: "甘い物 は 別腹" },
    EasyHint { seq: 2168350, kanji_split: "ペン は 剣 よりも 強し" },
    EasyHint { seq: 2176630, kanji_split: "千金 の 裘 は 一狐 の 腋 に 非ず" },
    EasyHint { seq: 2177220, kanji_split: "宝 さかって 入る 時 は さかって 出る" },
    EasyHint { seq: 2178680, kanji_split: "出す こと は 舌を出す も 嫌い" },
    EasyHint { seq: 2195810, kanji_split: "に 越した こと は ない" },
    EasyHint { seq: 2209690, kanji_split: "男 に 二言 は ない" },
    EasyHint { seq: 2216540, kanji_split: "背 に 腹 は 替えられぬ" },
    EasyHint { seq: 2219580, kanji_split: "寝言 は 寝て から 言え" },
    EasyHint { seq: 2411180, kanji_split: "用心する に 越した こと は ない" },
    EasyHint { seq: 2416580, kanji_split: "悪い 事 は 出来ぬ もの" },
    EasyHint { seq: 2416650, kanji_split: "衣 ばかり で 和尚 は できぬ" },
    EasyHint { seq: 2416680, kanji_split: "一銭 を 笑う 者 は 一銭 に 泣く" },
    EasyHint { seq: 2417140, kanji_split: "血 は 水 よりも 濃い" },
    EasyHint { seq: 2417180, kanji_split: "見る と 聞く とは 大違い" },
    EasyHint { seq: 2417270, kanji_split: "後 は 野となれ 山となれ" },
    EasyHint { seq: 2417540, kanji_split: "昨日 の 友 は 今日 の 敵" },
    EasyHint { seq: 2417580, kanji_split: "山椒 は 小粒 でも ぴりりと 辛い" },
    EasyHint { seq: 2417750, kanji_split: "蒔かぬ 種 は 生えぬ" },
    EasyHint { seq: 2417760, kanji_split: "鹿 を 追う 者 は 山 を 見ず" },
    EasyHint { seq: 2418060, kanji_split: "色事 は 思案 の 外" },
    EasyHint { seq: 2418100, kanji_split: "寝る ほど 楽 は なかりけり" },
    EasyHint { seq: 2418220, kanji_split: "人 の 口 に 戸 は 立てられず" },
    EasyHint { seq: 2418250, kanji_split: "人 は パン のみ にて 生くる 者 に 非ず" },
    EasyHint { seq: 2418270, kanji_split: "人 は 見かけ に よらぬ もの" },
    EasyHint { seq: 2418490, kanji_split: "世間 の 口 に 戸 は 立てられぬ" },
    EasyHint { seq: 2418500, kanji_split: "世間 は 広い 様 で 狭い" },
    EasyHint { seq: 2418600, kanji_split: "生ある 者 は 必ず 死あり" },
    EasyHint { seq: 2418700, kanji_split: "栴檀 は 双葉 より 芳し" },
    EasyHint { seq: 2418720, kanji_split: "前車 の 覆る は 後車 の 戒め" },
    EasyHint { seq: 2418900, kanji_split: "タダ より 高い もの は ない" },
    EasyHint { seq: 2418970, kanji_split: "知らぬ は 亭主 ばかり なり" },
    EasyHint { seq: 2419060, kanji_split: "釣り落とした 魚 は 大きい" },
    EasyHint { seq: 2419120, kanji_split: "転んでも ただ は 起きぬ" },
    EasyHint { seq: 2419190, kanji_split: "逃がした 魚 は 大きい" },
    EasyHint { seq: 2419260, kanji_split: "二兎 を 追う 者 は 一兎 をも 得ず" },
    EasyHint { seq: 2419270, kanji_split: "二度ある こと は 三度ある" },
    EasyHint { seq: 2419410, kanji_split: "馬鹿 と はさみ は 使いよう" },
    EasyHint { seq: 2419420, kanji_split: "馬鹿 に つける 薬 は ない" },
    EasyHint { seq: 2419450, kanji_split: "板子 一枚 下 は 地獄" },
    EasyHint { seq: 2419530, kanji_split: "夫婦喧嘩 は 犬 も 食わない" },
    EasyHint { seq: 2419690, kanji_split: "文 は 人 なり" },
    EasyHint { seq: 2419710, kanji_split: "聞く と 見る とは 大違い" },
    EasyHint { seq: 2419730, kanji_split: "便り の ない の は よい 便り" },
    EasyHint { seq: 2419800, kanji_split: "名の無い 星 は 宵 から 出る" },
    EasyHint { seq: 2419970, kanji_split: "余 の 辞書 に 不可能 という 文字 は ない" },
    EasyHint { seq: 2420010, kanji_split: "用心 に 越した こと は ない" },
    EasyHint { seq: 2420110, kanji_split: "例外 の ない 規則 は ない" },
    EasyHint { seq: 2420120, kanji_split: "歴史 は 繰り返す" },
    EasyHint { seq: 2420190, kanji_split: "驕る 平家 は 久しからず" },
    EasyHint { seq: 2424500, kanji_split: "目的 の ために は 手段 を 選ばない" },
    EasyHint { seq: 2442180, kanji_split: "命 に 別条 は ない" },
    EasyHint { seq: 2570900, kanji_split: "藁 で 束ねても 男 は 男" },
    EasyHint { seq: 2580730, kanji_split: "知 は 力 なり" },
    EasyHint { seq: 2582770, kanji_split: "既往 は 咎めず" },
    EasyHint { seq: 2641030, kanji_split: "相手 にとって 不足 は ない" },
    EasyHint { seq: 2694370, kanji_split: "転んでも ただ は 起きない" },
    EasyHint { seq: 2738830, kanji_split: "お客様 は 神様 です" },
    EasyHint { seq: 2757560, kanji_split: "宵越し の 銭 は 持たない" },
    EasyHint { seq: 2758920, kanji_split: "立っている もの は 親 でも 使え" },
    EasyHint { seq: 2761670, kanji_split: "先 の こと は 分からない" },
    EasyHint { seq: 2776820, kanji_split: "理屈 と 膏薬 は どこ へ でも つく" },
    EasyHint { seq: 2783090, kanji_split: "疑わしき は 罰せず" },
    EasyHint { seq: 2784400, kanji_split: "蟹 は 甲羅 に 似せて 穴 を 掘る" },
    EasyHint { seq: 2789240, kanji_split: "嘘 と 坊主 の 頭 は ゆった ことがない" },
    EasyHint { seq: 2792090, kanji_split: "正気 とは 思えない" },
    EasyHint { seq: 2797740, kanji_split: "一円 を 笑う 者 は 一円 に 泣く" },
    EasyHint { seq: 2798610, kanji_split: "お神酒 上がらぬ 神 は ない" },
    EasyHint { seq: 2826812, kanji_split: "悪い こと は 言わない" },
    EasyHint { seq: 2828308, kanji_split: "吐いた 唾 は 飲めぬ" },
    EasyHint { seq: 2830029, kanji_split: "明けない 夜 は ない" },
    EasyHint { seq: 2833956, kanji_split: "山 より 大きな 猪 は 出ぬ" },
    EasyHint { seq: 2833957, kanji_split: "老兵 は 死なず ただ 消え去る のみ" },
    EasyHint { seq: 2833961, kanji_split: "梅 は 食う とも 核 食う な 中 に 天神 寝てござる" },
    EasyHint { seq: 2833986, kanji_split: "悪 に 強い は 善 にも 強い" },
    EasyHint { seq: 2108440, kanji_split: "過ちて は 則ち 改むる に 憚る こと 勿れ" },
    EasyHint { seq: 2417420, kanji_split: "降り懸かる 火の粉 は 払わねば ならぬ" },
    EasyHint { seq: 2418640, kanji_split: "静かに 流れる 川 は 深い" },
    EasyHint { seq: 2835355, kanji_split: "無い 物 は 無い" },
    EasyHint { seq: 2835504, kanji_split: "気 は 確か" },
    EasyHint { seq: 2835673, kanji_split: "見る は 法楽" },
    EasyHint { seq: 2836181, kanji_split: "持つべき もの は 友" },
    EasyHint { seq: 2836183, kanji_split: "持つべき もの は 友人" },
    EasyHint { seq: 2836500, kanji_split: "経験者 は 語る" },
    EasyHint { seq: 2741060, kanji_split: "本日 は 晴天 なり" },
    EasyHint { seq: 2836784, kanji_split: "物 か は" },
    EasyHint { seq: 2837023, kanji_split: "明日 は 我が身" },
    EasyHint { seq: 2837133, kanji_split: "右 に 出る 者 は ない" },
    EasyHint { seq: 2839180, kanji_split: "便り が ない の は よい 便り" },
    EasyHint { seq: 2839838, kanji_split: "無理 は 禁物" },
    EasyHint { seq: 2839934, kanji_split: "元 は と言えば" },
    EasyHint { seq: 2840462, kanji_split: "すべて 世 は こと も なし" },
    EasyHint { seq: 2840493, kanji_split: "体 は 正直" },
    EasyHint { seq: 2840733, kanji_split: "話し上手 は 聞き上手" },
    EasyHint { seq: 2840752, kanji_split: "色男 金 と 力 は なかりけり" },
    EasyHint { seq: 2841085, kanji_split: "話 は 別" },
    EasyHint { seq: 2841164, kanji_split: "愛 は 盲目" },
    EasyHint { seq: 2841165, kanji_split: "恋 は 闇" },
    EasyHint { seq: 2841585, kanji_split: "礼 は はずむ" },
    EasyHint { seq: 2842361, kanji_split: "失敗 は 成功 の 母" },
    EasyHint { seq: 2843805, kanji_split: "細工 は 流々 仕上げ を 御覧じろ" },
    EasyHint { seq: 2843453, kanji_split: "立てば 芍薬 座れば 牡丹 歩く姿 は 百合 の 花" },
    EasyHint { seq: 2843281, kanji_split: "九層 の 台 は 累土 より 起こる" },
    EasyHint { seq: 2844718, kanji_split: "老いて は 益々壮ん なるべし" },
    EasyHint { seq: 2844721, kanji_split: "若い時 は 二度ない" },
    EasyHint { seq: 2844870, kanji_split: "習わぬ 経 は 読めぬ" },
    EasyHint { seq: 2844963, kanji_split: "避けて は 通れない" },
    EasyHint { seq: 2844990, kanji_split: "戴くもの は 夏 も 小袖" },
    EasyHint { seq: 2845002, kanji_split: "魚 は 頭 から 腐る" },
    EasyHint { seq: 2845919, kanji_split: "人 は 見目 より ただ 心" },
    EasyHint { seq: 2845920, kanji_split: "人 に 善言 を 与うる は 布帛 よりも 煖かなり" },
    EasyHint { seq: 2846470, kanji_split: "気 は 無い" },
    EasyHint { seq: 2847076, kanji_split: "虎 は 千里 往って 千里 還る" },
    EasyHint { seq: 2848309, kanji_split: "それ は ない" },
    EasyHint { seq: 2847626, kanji_split: "話 は 早い" },
    EasyHint { seq: 2848813, kanji_split: "美人 は 三日 で 飽きる" },
    EasyHint { seq: 2849042, kanji_split: "過去 は 過去" },
    EasyHint { seq: 2849859, kanji_split: "寝言 は 寝て 言え" },
    EasyHint { seq: 2851317, kanji_split: "困った 時 は お互い様" },
    EasyHint { seq: 2855884, kanji_split: "逃げた 魚 は 大きい" },
    EasyHint { seq: 2855675, kanji_split: "鯛 も 一人 は うまからず" },
    EasyHint { seq: 2856828, kanji_split: "ごめん で 済む なら 警察 は いらない" },
    EasyHint { seq: 2857339, kanji_split: "他 は 無い" },
    EasyHint { seq: 2857468, kanji_split: "無 から は 何も 生じない" },
    EasyHint { seq: 2861000, kanji_split: "今 は 亡き" },
    EasyHint { seq: 2861001, kanji_split: "今 は 無き" },
    EasyHint { seq: 2861231, kanji_split: "言って は なんです が" },
    EasyHint { seq: 2862670, kanji_split: "夫婦 は 合わせ鏡" },
    EasyHint { seq: 2863444, kanji_split: "然 は 然りながら" },
    EasyHint { seq: 2863557, kanji_split: "人生 は 一度きり" },
    EasyHint { seq: 2865369, kanji_split: "世間 は 張り物" },
    EasyHint { seq: 2867221, kanji_split: "武士 は 相身互い" },
    EasyHint { seq: 2868635, kanji_split: "止まない 雨 は ない" },
    EasyHint { seq: 2864666, kanji_split: "予定 は 未定" },
    EasyHint { seq: 2865149, kanji_split: "画像 は イメージ です" },
    EasyHint { seq: 2416600, kanji_split: "悪人 は 畳 の 上 で は 死ねない" },
    EasyHint { seq: 2767400, kanji_split: "鬼 は 外 福 は 内" },
    EasyHint { seq: 2418260, kanji_split: "人 は 一代 名 は 末代" },
    EasyHint { seq: 2828341, kanji_split: "花 は 桜木 人 は 武士" },
    EasyHint { seq: 2086560, kanji_split: "鶴 は 千年 亀 は 万年" },
    EasyHint { seq: 2152790, kanji_split: "楽 は 苦 の 種 苦 は 楽 の 種" },
    EasyHint { seq: 2154700, kanji_split: "旅 は 道連れ 世 は 情け" },
    EasyHint { seq: 2158840, kanji_split: "男 は 度胸 女 は 愛敬" },
    EasyHint { seq: 2168380, kanji_split: "沈黙 は 金 雄弁 は 銀" },
    EasyHint { seq: 2417120, kanji_split: "芸術 は 長く 人生 は 短し" },
    EasyHint { seq: 2417230, kanji_split: "言う は 易く 行う は 難し" },
    EasyHint { seq: 2417500, kanji_split: "今日 は 人の身 明日 は 我が身" },
    EasyHint { seq: 2417930, kanji_split: "女 は 弱し されど 母 は 強し" },
    EasyHint { seq: 2418180, kanji_split: "親 は 無くても 子 は 育つ" },
    EasyHint { seq: 2418550, kanji_split: "生 は 難く 死 は 易し" },
    EasyHint { seq: 2418630, kanji_split: "声 は すれども 姿 は 見えず" },
    EasyHint { seq: 2418650, kanji_split: "昔 は 昔 今 は 今" },
    EasyHint { seq: 2418740, kanji_split: "創業 は 易く 守成 は 難し" },
    EasyHint { seq: 2419150, kanji_split: "東 は 東 西 は 西" },
    EasyHint { seq: 2419950, kanji_split: "雄弁 は 銀 沈黙 は 金" },
    EasyHint { seq: 2419960, kanji_split: "夕焼け は 晴れ 朝焼け は 雨" },
    EasyHint { seq: 2420080, kanji_split: "旅 は 心 世 は 情け" },
    EasyHint { seq: 2424520, kanji_split: "去る者 は 追わず 来たる者 は 拒まず" },
    EasyHint { seq: 2558710, kanji_split: "遠き は 花 の 香 近き は 糞 の 香" },
    EasyHint { seq: 2719710, kanji_split: "フグ は 食いたし 命 は 惜しし" },
    EasyHint { seq: 2790690, kanji_split: "弓 は 袋 に 太刀 は 鞘" },
    EasyHint { seq: 2828900, kanji_split: "山中 の 賊 を 破る は 易く 心中 の 賊 を 破る は 難し" },
    EasyHint { seq: 2833976, kanji_split: "君子 は 周して 比せず 小人 は 比して 周せず" },
    EasyHint { seq: 2833959, kanji_split: "知る者 は 言わず 言う者 は 知らず" },
    EasyHint { seq: 2833900, kanji_split: "虎 は 死して 皮 を 留め 人 は 死して 名 を 残す" },
    EasyHint { seq: 2570040, kanji_split: "朝焼け は 雨 夕焼け は 晴れ" },
    EasyHint { seq: 2419570, kanji_split: "腹 が 減って は 戦 は 出来ぬ" },
    EasyHint { seq: 2255410, kanji_split: "浜 の 真砂 は 尽きるとも 世 に 盗人 の 種 は 尽きまじ" },
    EasyHint { seq: 2419720, kanji_split: "聞く は 一時 の 恥 聞かぬ は 末代 の 恥" },
    EasyHint { seq: 2419910, kanji_split: "問う は 一旦 の 恥 問わぬ は 末代 の 恥" },
    EasyHint { seq: 2757120, kanji_split: "問う は 一度 の 恥 問わぬ は 末代 の 恥" },
    EasyHint { seq: 2834642, kanji_split: "柳 は 緑 花 は 紅" },
    EasyHint { seq: 2836571, kanji_split: "聞く は 一時 の 恥 聞かぬ は 一生 の 恥" },
    EasyHint { seq: 2836731, kanji_split: "男 は 松 女 は 藤" },
    EasyHint { seq: 2839233, kanji_split: "転がる 石 に は 苔 は 付かない" },
    EasyHint { seq: 2835297, kanji_split: "此れ は 此れ は" },
    EasyHint { seq: 2845254, kanji_split: "上戸 は 毒 を 知らず 下戸 は 薬 を 知らず" },
    EasyHint { seq: 2845255, kanji_split: "文 は やりたし 書く手 は 持たぬ" },
    EasyHint { seq: 2845915, kanji_split: "旅 は 情け 人 は 心" },
    EasyHint { seq: 2845916, kanji_split: "人 は 人 我 は 我" },
    EasyHint { seq: 2847494, kanji_split: "行き は 良い良い 帰り は 怖い" },
    EasyHint { seq: 2848603, kanji_split: "始め は 処女 の 如く 後 は 脱兎 の 如し" },
    EasyHint { seq: 2153170, kanji_split: "目 には 目 を 歯 には 歯 を" },
    EasyHint { seq: 2422970, kanji_split: "人 には 添うて見よ 馬 には 乗って見よ" },
    EasyHint { seq: 2833500, kanji_split: "馬 には 乗って見よ 人 には 添うて見よ" },
    EasyHint { seq: 2857020, kanji_split: "居候 三杯目 には そっと出し" },
    EasyHint { seq: 2862061, kanji_split: "無下 には できない" },
    EasyHint { seq: 2863521, kanji_split: "上 には 上 が いる" },
    EasyHint { seq: 2863544, kanji_split: "右に出る 者 は いない" },
    EasyHint { seq: 2863602, kanji_split: "余り物 には 福 が ある" },
    EasyHint { seq: 1008660, kanji_split: "隣 の 芝生 は 青い" },
    EasyHint { seq: 1204760, kanji_split: "蛙 の 子 は 蛙" },
    EasyHint { seq: 2113380, kanji_split: "金 は 天下 の 回り物" },
    EasyHint { seq: 2141020, kanji_split: "秋の日 は 釣瓶落とし" },
    EasyHint { seq: 2144050, kanji_split: "秋 の 鹿 は 笛 に 寄る" },
    EasyHint { seq: 2152870, kanji_split: "柳 の 下 に いつも 泥鰌 は おらぬ" },
    EasyHint { seq: 2152930, kanji_split: "一年 の 計 は 元旦 に あり" },
    EasyHint { seq: 2158240, kanji_split: "若い 時 の 苦労 は 買うてもせよ" },
    EasyHint { seq: 2202070, kanji_split: "勝負 は 時 の 運" },
    EasyHint { seq: 2227110, kanji_split: "カエサル の もの は カエサル に" },
    EasyHint { seq: 2417800, kanji_split: "蛇 の 道 は 蛇" },
    EasyHint { seq: 2418170, kanji_split: "親 の 光 は 七光り" },
    EasyHint { seq: 2420070, kanji_split: "旅 の 恥 は 掻き捨て" },
    EasyHint { seq: 2420100, kanji_split: "隣 の 花 は 赤い" },
    EasyHint { seq: 2582990, kanji_split: "狐 の 子 は 頬白" },
    EasyHint { seq: 2697510, kanji_split: "君父 の 讐 は 倶に 天 を 戴かず" },
    EasyHint { seq: 2827732, kanji_split: "若い 時 の 苦労 は 買ってもせよ" },
    EasyHint { seq: 2835925, kanji_split: "煩悩 の 犬 は 追えども 去らず" },
    EasyHint { seq: 2174750, kanji_split: "己 の 欲せざる 所 は 人 に 施す 勿れ" },
    EasyHint { seq: 2838865, kanji_split: "だけ の 事 は ある" },
    EasyHint { seq: 2838606, kanji_split: "今日 の ところ は" },
    EasyHint { seq: 2838426, kanji_split: "木 の 実 は 本 へ 落つ" },
    EasyHint { seq: 2845252, kanji_split: "下戸 の 建てた 蔵 は ない" },
    EasyHint { seq: 2858678, kanji_split: "自分 の こと は 棚 に 上げる" },
    EasyHint { seq: 2859764, kanji_split: "他人 の 不幸 は 蜜 の 味" },
    EasyHint { seq: 2860668, kanji_split: "親 の 恩 は 子 で 送る" },
    EasyHint { seq: 2867148, kanji_split: "敵 の 急所 は 我が 急所" },
    EasyHint { seq: 1487700, kanji_split: "必要 は 発明 の 母" },
    EasyHint { seq: 1320150, kanji_split: "失敗 は 成功 の もと" },
    EasyHint { seq: 2126750, kanji_split: "悪妻 は 百年 の 不作" },
    EasyHint { seq: 2141010, kanji_split: "逢う は 別れ の 始め" },
    EasyHint { seq: 2144040, kanji_split: "商い は 牛 の よだれ" },
    EasyHint { seq: 2152780, kanji_split: "苦 は 楽 の 種" },
    EasyHint { seq: 2211780, kanji_split: "情け は 人 の 為 ならず" },
    EasyHint { seq: 2416720, kanji_split: "嘘つき は 泥棒 の 始まり" },
    EasyHint { seq: 2416970, kanji_split: "教うる は 学ぶ の 半ば" },
    EasyHint { seq: 2417350, kanji_split: "口 は 禍 の 元" },
    EasyHint { seq: 2417610, kanji_split: "子 は 三界 の 首枷" },
    EasyHint { seq: 2417810, kanji_split: "弱き者 よ 汝 の 名 は 女 也" },
    EasyHint { seq: 2418340, kanji_split: "人間 は 万物 の 霊長" },
    EasyHint { seq: 2418470, kanji_split: "据え膳 食わぬ は 男 の 恥" },
    EasyHint { seq: 2418540, kanji_split: "正直 は 一生 の 宝" },
    EasyHint { seq: 2418590, kanji_split: "生兵法 は 大怪我 の もと" },
    EasyHint { seq: 2419020, kanji_split: "朝起き は 三文 の 徳" },
    EasyHint { seq: 2420140, kanji_split: "恋 は 思案 の 外" },
    EasyHint { seq: 2550210, kanji_split: "幸運 の 女神 は 前髪 しかない" },
    EasyHint { seq: 2591070, kanji_split: "火事 と 喧嘩 は 江戸 の 華" },
    EasyHint { seq: 2796370, kanji_split: "禍福 は 糾える 縄 の ごとし" },
    EasyHint { seq: 2833968, kanji_split: "人間 は 万物 の 尺度 である" },
    EasyHint { seq: 2833958, kanji_split: "言葉 は 身 の 文" },
    EasyHint { seq: 2832652, kanji_split: "挨拶 は 時 の 氏神" },
    EasyHint { seq: 2111130, kanji_split: "早起き は 三文 の 徳" },
    EasyHint { seq: 2417830, kanji_split: "酒 は 百薬 の 長" },
    EasyHint { seq: 2837015, kanji_split: "落ち武者 は 薄 の 穂 に 怖ず" },
    EasyHint { seq: 2837756, kanji_split: "風邪 は 万病 の 元" },
    EasyHint { seq: 2842831, kanji_split: "口 は 災い の 門" },
    EasyHint { seq: 2843962, kanji_split: "生 は 死 の 始め" },
    EasyHint { seq: 2853754, kanji_split: "正直 は 最善 の 策" },
    EasyHint { seq: 2860665, kanji_split: "子 は 親 の 鏡" },
    EasyHint { seq: 2860666, kanji_split: "子供 は 親 の 鏡" },
    EasyHint { seq: 2860667, kanji_split: "子供 は 風 の 子" },
    EasyHint { seq: 2860677, kanji_split: "兄弟 は 他人 の 始まり" },
    EasyHint { seq: 1213500, kanji_split: "甘言 は 偶人 を 喜ばす" },
    EasyHint { seq: 1470130, kanji_split: "能 ある 鷹 は 爪 を 隠す" },
    EasyHint { seq: 1929200, kanji_split: "悪貨 は 良貨 を 駆逐する" },
    EasyHint { seq: 2077530, kanji_split: "類 は 友 を 呼ぶ" },
    EasyHint { seq: 2079030, kanji_split: "大 は 小 を 兼ねる" },
    EasyHint { seq: 2089460, kanji_split: "鳴く 猫 は 鼠 を 捕らぬ" },
    EasyHint { seq: 2102600, kanji_split: "おぼれる 者 は わら をも つかむ" },
    EasyHint { seq: 2168340, kanji_split: "天 は 自ら 助くる 者 を 助く" },
    EasyHint { seq: 2416900, kanji_split: "急いて は 事 を 仕損ずる" },
    EasyHint { seq: 2417110, kanji_split: "芸 は 身 を 助く" },
    EasyHint { seq: 2419100, kanji_split: "天 は 二物 を 与えず" },
    EasyHint { seq: 2419810, kanji_split: "名 は 体 を 表す" },
    EasyHint { seq: 2520680, kanji_split: "義 を 見てせざる は 勇なきなり" },
    EasyHint { seq: 2627320, kanji_split: "急いて は 事 を 仕損じる" },
    EasyHint { seq: 2686140, kanji_split: "大人 は 赤子 の 心 を 失わず" },
    EasyHint { seq: 2833952, kanji_split: "足る を 知る 者 は 富む" },
    EasyHint { seq: 2832631, kanji_split: "井蛙 は 以って 海 を 語る 可からず" },
    EasyHint { seq: 2832604, kanji_split: "良禽 は 木 を 択んで棲む" },
    EasyHint { seq: 2757650, kanji_split: "日光 を 見ない 中 は 結構 と言う な" },
    EasyHint { seq: 2419440, kanji_split: "敗軍 の 将 は 兵 を 語らず" },
    EasyHint { seq: 2834645, kanji_split: "飢えたる 犬 は 棒 を 恐れず" },
    EasyHint { seq: 2836094, kanji_split: "満 は 損 を 招く" },
    EasyHint { seq: 2844015, kanji_split: "大徳 は 小怨 を 滅す" },
    EasyHint { seq: 2844292, kanji_split: "氷 は 水 より 出でて 水 よりも 寒し" },
    EasyHint { seq: 2845250, kanji_split: "芸 は 身 を 助ける" },
    EasyHint { seq: 2845917, kanji_split: "我 は 仮説 を 作らず" },
    EasyHint { seq: 2845918, kanji_split: "歌人 は 居ながらにして 名所 を 知る" },
    EasyHint { seq: 2846531, kanji_split: "老いたる 馬 は 道 を 忘れず" },
    EasyHint { seq: 2847018, kanji_split: "名人 は 人 を 謗らず" },
    EasyHint { seq: 2850060, kanji_split: "赤き は 酒 の 咎" },
    EasyHint { seq: 2850189, kanji_split: "君子 の 交わり は 淡きこと 水 の ごとし" },
    EasyHint { seq: 2855699, kanji_split: "謀 は 密なる を 良しとす" },
    EasyHint { seq: 2859193, kanji_split: "百里 を 行く 者 は 九十 を 半ばとす" },
    EasyHint { seq: 2859070, kanji_split: "獅子 は 我が子 を 千尋 の 谷 に 落とす" },
    EasyHint { seq: 2860664, kanji_split: "子供 は 親 の 背中 を 見て 育つ" },
    EasyHint { seq: 2095170, kanji_split: "天才 と 狂人 は 紙一重" },
    EasyHint { seq: 2237240, kanji_split: "女房 と 畳 は 新しい 方がいい" },
    EasyHint { seq: 2835775, kanji_split: "今 となって は" },
    EasyHint { seq: 2847205, kanji_split: "バカ と 煙 は 高い 所 へ 上る" },
    EasyHint { seq: 2847632, kanji_split: "下戸 と 化け物 は ない" },
    EasyHint { seq: 2124980, kanji_split: "そう は 問屋 が 卸さない" },
    EasyHint { seq: 2395080, kanji_split: "過ぎたる は 及ばざる が ごとし" },
    EasyHint { seq: 2395090, kanji_split: "過ぎたる は 猶及ばざる が 如し" },
    EasyHint { seq: 2417400, kanji_split: "慌てる 乞食 は もらい が 少ない" },
    EasyHint { seq: 2419860, kanji_split: "明日 は 明日 の 風 が 吹く" },
    EasyHint { seq: 2852239, kanji_split: "犬 が 西 向きゃ 尾 は 東" },
    EasyHint { seq: 2852243, kanji_split: "雨 の 降る 日 は 天気 が 悪い" },
    EasyHint { seq: 2862433, kanji_split: "商人 は 損して いつか 倉 が 建つ" },
    EasyHint { seq: 2863524, kanji_split: "まさか とは 思う が" },
    EasyHint { seq: 2864960, kanji_split: "言い方 は 悪いです が" },
    EasyHint { seq: 2138600, kanji_split: "百聞 は 一見 に しかず" },
    EasyHint { seq: 2153120, kanji_split: "良薬 は 口 に 苦し" },
    EasyHint { seq: 2153130, kanji_split: "目 は 口 ほど に 物を言う" },
    EasyHint { seq: 2168390, kanji_split: "鉄 は 熱い うち に 打て" },
    EasyHint { seq: 2171910, kanji_split: "去る者 は 日々 に 疎し" },
    EasyHint { seq: 2416560, kanji_split: "ローマ は 一日 に して 成らず" },
    EasyHint { seq: 2416730, kanji_split: "運 は 天 に 在り" },
    EasyHint { seq: 2416800, kanji_split: "火 の ない ところ に 煙 は 立たない" },
    EasyHint { seq: 2417160, kanji_split: "健全なる 精神 は 健全なる 身体 に 宿る" },
    EasyHint { seq: 2417390, kanji_split: "孝行 の したい 時分 に 親 は なし" },
    EasyHint { seq: 2418120, kanji_split: "新しい 酒 は 古い 革袋 に 入れる" },
    EasyHint { seq: 2418130, kanji_split: "深い 川 は 静かに 流れる" },
    EasyHint { seq: 2418450, kanji_split: "水 は 低きに 流る" },
    EasyHint { seq: 2418750, kanji_split: "すべて の 道 は ローマ に 通ず" },
    EasyHint { seq: 2419080, kanji_split: "鉄 は 熱い うち に 鍛え よ" },
    EasyHint { seq: 2420150, kanji_split: "老いて は 子 に 従え" },
    EasyHint { seq: 2566010, kanji_split: "秋茄子 は 嫁 に 食わす な" },
    EasyHint { seq: 2832573, kanji_split: "巧詐 は 拙誠 に 如かず" },
    EasyHint { seq: 2704850, kanji_split: "花泥棒 は 罪 に ならない" },
    EasyHint { seq: 2837518, kanji_split: "文 は 武 に 勝る" },
    EasyHint { seq: 2837552, kanji_split: "巧遅 は 拙速 に 如かず" },
    EasyHint { seq: 2842829, kanji_split: "悪名 は 無名 に 勝る" },
    EasyHint { seq: 2845256, kanji_split: "志 は 松 の 葉 に 包め" },
    EasyHint { seq: 2845443, kanji_split: "天災 は 忘れた頃 に やってくる" },
    EasyHint { seq: 2846430, kanji_split: "凝って は 思案 に 能わず" },
    EasyHint { seq: 2851107, kanji_split: "女 は 三界 に 家 なし" },
    EasyHint { seq: 2855268, kanji_split: "秋刀魚 は 目黒 に 限る" },
    EasyHint { seq: 2859282, kanji_split: "子供 は 三歳 までに 一生分 の 親孝行 を する" },
    EasyHint { seq: 2854538, kanji_split: "志ある者 は 事 竟に 成る" },
    EasyHint { seq: 2868513, kanji_split: "人 は パン のみ にて 生くる に 非ず" },
    EasyHint { seq: 2418150, kanji_split: "親 に 似ぬ 子 は 鬼子" },
    EasyHint { seq: 2419940, kanji_split: "柳 の 下 に 何時も 泥鰌 は 居ない" },
    EasyHint { seq: 2832738, kanji_split: "身体髪膚 これ を 父母 に 受くあえて 毀傷せざる は 孝 の 始めなり" },
    EasyHint { seq: 2834655, kanji_split: "親 の 意見 と 茄子 の 花 は 千 に 一つ も 無駄 は ない" },
    EasyHint { seq: 2830412, kanji_split: "他 に 方法 は 無い" },
    EasyHint { seq: 2666530, kanji_split: "墓 に 布団 は 着せられぬ" },
    EasyHint { seq: 2847238, kanji_split: "知らん が 為に 我 は 信ず" },
    EasyHint { seq: 2854601, kanji_split: "事 と 次第 によって は" },
    EasyHint { seq: 2855600, kanji_split: "武士 に 二言 は ない" },
    EasyHint { seq: 2863450, kanji_split: "人の口 に 戸 は 立てられぬ" },
    EasyHint { seq: 2864443, kanji_split: "に 至って は" },
    EasyHint { seq: 2204530, kanji_split: "ヘブライ人 へ の 手紙" },
    EasyHint { seq: 2813120, kanji_split: "ヘブル人 へ の 手紙" },
    EasyHint { seq: 2839843, kanji_split: "上 を 下 へ" },
    EasyHint { seq: 2839846, kanji_split: "上 や 下 へ の 大騒ぎ" },
    EasyHint { seq: 2841303, kanji_split: "足下 へ も 寄りつけない" },
    EasyHint { seq: 1151370, kanji_split: "悪 の 道 へ 誘う" },
    EasyHint { seq: 1171020, kanji_split: "右 から 左 へ" },
    EasyHint { seq: 1898770, kanji_split: "中 へ 入る" },
    EasyHint { seq: 2125750, kanji_split: "そこ へ 持ってきて" },
    EasyHint { seq: 2129780, kanji_split: "目 から 鼻 へ 抜ける" },
    EasyHint { seq: 2177720, kanji_split: "棚 へ 上げる" },
    EasyHint { seq: 2402730, kanji_split: "故郷 へ 錦 を 飾る" },
    EasyHint { seq: 2431220, kanji_split: "への字 に 結んだ 口" },
    EasyHint { seq: 2515280, kanji_split: "力 へ の 意志" },
    EasyHint { seq: 2515290, kanji_split: "権力 へ の 意志" },
    EasyHint { seq: 2716340, kanji_split: "平均 へ の 回帰" },
    EasyHint { seq: 2738180, kanji_split: "右 へ 倣え" },
    EasyHint { seq: 2826689, kanji_split: "東 へ 東 へ" },
    EasyHint { seq: 2831475, kanji_split: "脇 へ それる" },
    EasyHint { seq: 2219570, kanji_split: "元 へ" },
    EasyHint { seq: 2017030, kanji_split: "次 から 次 へ と" },
    EasyHint { seq: 2102190, kanji_split: "上 を 下 へ の 大騒ぎ" },
    EasyHint { seq: 2845308, kanji_split: "寺 から 里 へ" },
    EasyHint { seq: 2849371, kanji_split: "何処 へ やら" },
    EasyHint { seq: 2204790, kanji_split: "コリント人 へ の 手紙" },
    EasyHint { seq: 2204800, kanji_split: "ガラテヤ人 へ の 手紙" },
    EasyHint { seq: 2204840, kanji_split: "ローマ人 へ の 手紙" },
    EasyHint { seq: 2859067, kanji_split: "そこ へ 行く と" },
    EasyHint { seq: 1586550, kanji_split: "後 へ 引く" },
    EasyHint { seq: 2834606, kanji_split: "親 は 無く とも 子 は 育つ" },
    EasyHint { seq: 2834583, kanji_split: "病 は 口 より 入り 禍 は 口 より 出ず" },
    EasyHint { seq: 2834582, kanji_split: "安物 は 高物" },
    EasyHint { seq: 2834576, kanji_split: "目 は 心 の 鏡" },
    EasyHint { seq: 2834575, kanji_split: "目 は 心 の 窓" },
    EasyHint { seq: 2834651, kanji_split: "冷や酒 と 親 の 意見 は 後 で きく" },
    EasyHint { seq: 2834564, kanji_split: "火 の 無い ところ に 煙 は 立たなぬ" },
    EasyHint { seq: 2834563, kanji_split: "徳 は 孤 ならず 必ず 隣 あり" },
    EasyHint { seq: 2834560, kanji_split: "君子 は 器 ならず" },
    EasyHint { seq: 2834416, kanji_split: "馬鹿 は 風邪 を 引かない" },
    EasyHint { seq: 2834363, kanji_split: "墨 は 餓鬼 に 磨らせ 筆 は 鬼 に 持たせよ" },
    EasyHint { seq: 2834360, kanji_split: "信 は 荘厳 より 起こる" },
    EasyHint { seq: 2834321, kanji_split: "父母 の 恩 は 山 よりも 高く 海 よりも 深し" },
    EasyHint { seq: 2834318, kanji_split: "二人 は 伴侶 三人 は 仲間割れ" },
    EasyHint { seq: 2834316, kanji_split: "人 の 花 は 赤い" },
    EasyHint { seq: 2834313, kanji_split: "紅 は 園生 に 植えても 隠れなし" },
    EasyHint { seq: 2834310, kanji_split: "地獄 は 壁一重" },
    EasyHint { seq: 2834309, kanji_split: "人 は 死して 名 を 留む" },
    EasyHint { seq: 2834308, kanji_split: "浮世 は 夢" },
    EasyHint { seq: 2834287, kanji_split: "隣 の 芝 は 青い" },
    EasyHint { seq: 2834263, kanji_split: "弱き者 汝 の 名 は 女 なり" },
    EasyHint { seq: 2834244, kanji_split: "知識 は 力 なり" },
    EasyHint { seq: 2834239, kanji_split: "武士 は 食わねど 高楊枝" },
    EasyHint { seq: 2834233, kanji_split: "貧 は 世界 の 福 の 神" },
    EasyHint { seq: 2834232, kanji_split: "光 は 東 から" },
    EasyHint { seq: 2834228, kanji_split: "馬 に 乗る まで は 牛 に 乗れ" },
    EasyHint { seq: 2834227, kanji_split: "言わぬ は 言う に 優る" },
    EasyHint { seq: 2834224, kanji_split: "合わせ物 は 離れ物" },
    EasyHint { seq: 2834220, kanji_split: "悪 は 延べよ" },
    EasyHint { seq: 2833980, kanji_split: "牛 は 牛連れ 馬 は 馬連れ" },
    EasyHint { seq: 2833979, kanji_split: "馬鹿 と 天才 は 紙一重" },
    EasyHint { seq: 2833939, kanji_split: "長い物 には 巻かれ よ" },
    EasyHint { seq: 2833938, kanji_split: "長い物 には 巻かれろ" },
    EasyHint { seq: 2842906, kanji_split: "ない 訳 には 行かない" },
    EasyHint { seq: 2835463, kanji_split: "人目 も はばからず" },
    EasyHint { seq: 2849004, kanji_split: "ギョエテ とは 俺 の こと か と ゲーテ いい" },
];

/// Lazy index of [`EASY_HINTS`] keyed by `seq` — mirrors the upstream
/// `*hint-map*` hashtable's O(1) lookup. Populated on first call to
/// [`hint_map_dispatch`]; static lifetime borrowed from the [`EASY_HINTS`]
/// table itself, so no clone of the entry data.
fn easy_hints_by_seq() -> &'static HashMap<i32, &'static EasyHint> {
    static CACHE: OnceLock<HashMap<i32, &'static EasyHint>> = OnceLock::new();
    CACHE.get_or_init(|| EASY_HINTS.iter().map(|e| (e.seq, e)).collect())
}

/// Three-state outcome of `*hint-map*` lookup + dispatch. The
/// distinction between [`Self::Unregistered`] and
/// [`Self::Registered(None)`] is load-bearing: upstream
/// `(if hint-fn (funcall hint-fn reading) (loop ...))` at
/// `dict-split.lisp:941-945` walks the conj-of fallback ONLY when
/// the primary seq is unregistered. A registered-but-test-failed
/// primary returns nil to its caller — it does NOT trigger the
/// conj-of walk.
#[derive(Debug)]
pub enum HintDispatch {
    /// Primary seq has no entry in `*hint-map*`. Caller should
    /// fall through to the conj-of walk.
    Unregistered,
    /// Primary seq is registered. The body ran and produced either
    /// a hinted kana string (`Some`) or nil (`None`, from a failed
    /// `:test` clause or an `:from-end` search that didn't find its
    /// substring). Either way, this is the final answer for this
    /// seq — no conj-of walk.
    Registered(Option<String>),
}

/// Dispatcher across `*hint-map*`. Hash lookup against
/// [`easy_hints_by_seq`] followed by a `match` on the 17 simple-hint
/// seq groups. Returns [`HintDispatch::Unregistered`] when no entry
/// matches; otherwise [`HintDispatch::Registered`] with the body's
/// result.
pub async fn hint_map_dispatch(
    ctx: &KaniranContext,
    seq: i32,
    reading: &KaniWordDispatchEnum,
) -> Result<HintDispatch, sqlx::Error> {
    // Easy-hints are checked first because they come after all
    // def-simple-hint forms in dict-split.lisp source, so their
    // `(setf (gethash ...))` overrides earlier registrations for
    // the same seq. (See module doc § "Order semantics".)
    if let Some(entry) = easy_hints_by_seq().get(&seq).copied() {
        let result = run_easy_hint(ctx, entry, reading).await?;
        return Ok(HintDispatch::Registered(result));
    }

    simple_hint_dispatch(ctx, seq, reading).await
}

async fn simple_hint_dispatch(
    ctx: &KaniranContext,
    seq: i32,
    reading: &KaniWordDispatchEnum,
) -> Result<HintDispatch, sqlx::Error> {
    // Each match arm returns the `Option<String>` produced by its
    // hint body (Some = test-pass with hinted kana; None =
    // `:test` clause failed or `:from-end` search missed). Both
    // are HintDispatch::Registered because the seq IS in *hint-map*.
    // Only the catchall `_` arm — seq not in *hint-map* —
    // returns HintDispatch::Unregistered so the caller can fall
    // through to the conj-of walk per `dict-split.lisp:941-945`.
    let result: Option<String> = match seq {
        // dict-split.lisp:1014 (def-simple-hint (2028920 2029000) (l) (:mod (- l 1)))
        2028920 | 2029000 => {
            let Some((_kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [safe_hint(KaniHintKind::Mod, l - 1)]
                .into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1021 (def-simple-hint ;; no space — (l k)
        //   (:test (alexandria:ends-with #\は k)) (:mod (- l 1)))
        1289480 | 1289400 | 1008450 | 2215430 | 2028950 => {
            let Some((kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            if !ends_with_char(&kana, 'は') {
                return Ok(HintDispatch::Registered(None));
            }
            let hints: Vec<_> = [safe_hint(KaniHintKind::Mod, l - 1)]
                .into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1032 (def-simple-hint ;; with space — (l k)
        //   (:test (alexandria:ends-with #\は k))
        //   (:space (- l 1)) (:mod (- l 1)))
        1006660 | 1008500 | 1307530 | 1320830 | 1324320 | 1524990 | 1586850
        | 1877880 | 1897510 | 1907300 | 1912570 | 2034440 | 2098160 | 2105820
        | 2134680 | 2136300 | 2176280 | 2177410 | 2177420 | 2177430 | 2177440
        | 2177450 | 2256430 | 2428890 | 2523450 | 2557290 | 2673120 | 2691570
        | 2702090 | 2717440 | 2717510 | 2828541 | 1217970 | 1331520 | 1907290
        | 1914670 | 1950430 | 2136680 | 2181810 | 2181730 | 2576840 | 1331510
        | 1010470 | 2008290 | 2136690 | 2829815 | 2830216 | 2840063 | 2841096
        | 2841959 | 2844687 | 2844836 | 2850535 | 2861249 => {
            let Some((kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            if !ends_with_char(&kana, 'は') {
                return Ok(HintDispatch::Registered(None));
            }
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, l - 1),
                safe_hint(KaniHintKind::Mod, l - 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1098 (def-simple-hint (2844416) (l k)
        //   (:space (- l 1)) (:mod 0))
        2844416 => {
            let Some((_kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, l - 1),
                safe_hint(KaniHintKind::Mod, 0),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1104 (def-simple-hint (2097010 1009150) (l)
        //   (:space (- l 1)) (:mod (- l 1)))
        2097010 | 1009150 => {
            let Some((_kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, l - 1),
                safe_hint(KaniHintKind::Mod, l - 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1112 (def-simple-hint (2261800) (l)
        //   (:space 2) (:mod 2) (:space 3) (:space (- l 1)) (:mod (- l 1)))
        2261800 => {
            let Some((_kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, 2),
                safe_hint(KaniHintKind::Mod, 2),
                safe_hint(KaniHintKind::Space, 3),
                safe_hint(KaniHintKind::Space, l - 1),
                safe_hint(KaniHintKind::Mod, l - 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1122 (def-simple-hint ;; では/には ending — (l k)
        //   (:test (alexandria:ends-with #\は k))
        //   (:space (- l 2)) (:mod (- l 1)))
        1009480 | 1315860 | 1406050 | 2026610 | 2061740 | 2097310 | 2101020
        | 2119920 | 2134700 | 2200100 | 2407650 | 2553140 | 2762790 | 1288910
        | 1423320 | 2099850 | 1006890 => {
            let Some((kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            if !ends_with_char(&kana, 'は') {
                return Ok(HintDispatch::Registered(None));
            }
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, l - 2),
                safe_hint(KaniHintKind::Mod, l - 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1148 (def-simple-hint — (l k)
        //   (deha (search "では" k :from-end t))
        //   (:mod (1+ deha)))
        2089020 | 2823770 | 2098240 | 2027020 | 2135480 | 2397760 | 2724540
        | 2757720 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(deha) = search_chars("では", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [safe_hint(KaniHintKind::Mod, deha as i64 + 1)]
                .into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1162 (def-simple-hint ;; ends with ではない — (l k)
        //   (deha (search "では" k :from-end t))
        //   (:space deha) (:mod (1+ deha)))
        2027080 | 2126160 | 2126140 | 2131120 | 2136640 | 2214830 | 2221680
        | 2416950 | 2419210 | 2664520 | 2682500 | 2775790 | 1343120 | 2112270
        | 2404260 | 2758400 | 2827556 | 2057560 | 2841318 | 2088970 | 2833095
        | 2835662 | 2841608 | 2841609 | 2845739 | 2849457 | 2850045 | 2854412 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(deha) = search_chars("では", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, deha as i64),
                safe_hint(KaniHintKind::Mod, deha as i64 + 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1199 (def-simple-hint ;; では in the middle — (l k)
        //   (deha (search "では" k :from-end t))
        //   (:space deha) (:mod (1+ deha)) (:space (+ 2 deha)))
        2037860 | 2694350 | 2111220 | 2694360 | 2182700 | 2142010 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(deha) = search_chars("では", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, deha as i64),
                safe_hint(KaniHintKind::Mod, deha as i64 + 1),
                safe_hint(KaniHintKind::Space, deha as i64 + 2),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1214 (def-simple-hint ;; には in the middle — (l k)
        //   (niha (search "には" k :from-end t))
        //   (:space niha) (:mod (1+ niha)) (:space (+ 2 niha)))
        2057580 | 2067990 | 2103020 | 2105980 | 2152700 | 2416920 | 2418030
        | 2792210 | 2792420 | 2417920 | 2598720 | 2420170 | 2597190 | 2597800
        | 2057570 | 2419360 | 2121480 | 2646440 | 2740880 | 2416860 | 2156910
        | 2182690 | 2848157 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(niha) = search_chars("には", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, niha as i64),
                safe_hint(KaniHintKind::Mod, niha as i64 + 1),
                safe_hint(KaniHintKind::Space, niha as i64 + 2),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1247 (def-simple-hint ;; starts with には/とは — (l k)
        //   (:mod 1) (:space 2))
        2181860 | 2037320 | 2125460 | 2128060 | 2070730 => {
            let Some((_kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Mod, 1),
                safe_hint(KaniHintKind::Space, 2),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1258 (def-simple-hint (2832044) — (l k)
        //   (niha (search "には" k :from-end t))
        //   (:space niha) (:mod (1+ niha)) (:space (+ 2 niha))
        //   (:space (- l 1)))
        2832044 => {
            let Some((kana, l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(niha) = search_chars("には", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, niha as i64),
                safe_hint(KaniHintKind::Mod, niha as i64 + 1),
                safe_hint(KaniHintKind::Space, niha as i64 + 2),
                safe_hint(KaniHintKind::Space, l - 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1269 (def-simple-hint ;; は in the middle — (l k)
        //   (ha (search "は" k :from-end t))
        //   (:space ha) (:mod ha) (:space (1+ ha)))
        1008970 | 1188440 | 1193090 | 1394290 | 1855940 | 1949380 | 1981600
        | 1982230 | 2018320 | 2062980 | 2078930 | 2089520 | 2089620 | 2098150
        | 2108910 | 2115570 | 2118120 | 2118430 | 2118440 | 2134480 | 2135530
        | 2136710 | 2141360 | 2168360 | 2173880 | 2174570 | 2176450 | 2177240
        | 2200690 | 2210960 | 2213470 | 2255320 | 2275900 | 2403520 | 2408680
        | 2416870 | 2416930 | 2417040 | 2417150 | 2417980 | 2418090 | 2418280
        | 2418800 | 2418840 | 2418920 | 2419030 | 2419350 | 2419390 | 2419590
        | 2419600 | 2419610 | 2419620 | 2420180 | 2583560 | 2585230 | 2593830
        | 2600530 | 2618920 | 2618990 | 2708230 | 2716900 | 2737650 | 2741810
        | 2744840 | 2827754 | 2831359 | 2833597 | 2839953 | 2844002 | 2858918
        | 2862330 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(ha) = search_chars("は", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, ha as i64),
                safe_hint(KaniHintKind::Mod, ha as i64),
                safe_hint(KaniHintKind::Space, ha as i64 + 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1348 (def-simple-hint — (l k)
        //   (ha (search "は" k))   ;; NB: no :from-end
        //   (:space ha) (:mod ha) (:space (1+ ha)))
        2867144 | 2867149 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(ha) = search_chars("は", &kana, false) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, ha as i64),
                safe_hint(KaniHintKind::Mod, ha as i64),
                safe_hint(KaniHintKind::Space, ha as i64 + 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1361 (def-simple-hint (2716860)
        //   ;; "そう は イカ の 金玉" — (l k)
        //   (ha (search "は" k)) (no (search "の" k :from-end t))
        //   (:space ha) (:mod ha) (:space (1+ ha))
        //   (:space no) (:space (1+ no)))
        2716860 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(ha) = search_chars("は", &kana, false) else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(no) = search_chars("の", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, ha as i64),
                safe_hint(KaniHintKind::Mod, ha as i64),
                safe_hint(KaniHintKind::Space, ha as i64 + 1),
                safe_hint(KaniHintKind::Space, no as i64),
                safe_hint(KaniHintKind::Space, no as i64 + 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // dict-split.lisp:1371 (def-simple-hint (2845260)
        //   ;; "他所 は 他所 うち は うち" — (l k)
        //   (ha1 (search "は" k))
        //   (ha2 (search "は" k :from-end t))
        //   (uu (search "う" k))
        //   (:space ha1) (:mod ha1) (:space (1+ ha1))
        //   (:space uu) (:space ha2) (:mod ha2) (:space (1+ ha2)))
        2845260 => {
            let Some((kana, _l)) = true_kana_and_len(ctx, reading).await? else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(ha1) = search_chars("は", &kana, false) else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(ha2) = search_chars("は", &kana, true) else {
                return Ok(HintDispatch::Registered(None));
            };
            let Some(uu) = search_chars("う", &kana, false) else {
                return Ok(HintDispatch::Registered(None));
            };
            let hints: Vec<_> = [
                safe_hint(KaniHintKind::Space, ha1 as i64),
                safe_hint(KaniHintKind::Mod, ha1 as i64),
                safe_hint(KaniHintKind::Space, ha1 as i64 + 1),
                safe_hint(KaniHintKind::Space, uu as i64),
                safe_hint(KaniHintKind::Space, ha2 as i64),
                safe_hint(KaniHintKind::Mod, ha2 as i64),
                safe_hint(KaniHintKind::Space, ha2 as i64 + 1),
            ].into_iter().flatten().collect();
            finish_simple_hint(ctx, reading, hints).await?
        }

        // Not in the upstream *hint-map* — seq is unregistered.
        // Return immediately so the outer wrapper doesn't classify
        // this as Registered.
        _ => return Ok(HintDispatch::Unregistered),
    };
    Ok(HintDispatch::Registered(result))
}

// =========================================================================
// *kana-hint-space* (dict-split.lisp:814)
// =========================================================================

/// Sentinel character marking hint-injected spaces in kana strings.
/// Used by [`HINT_CHAR_MAP`] and [`hint_simplify_map`] to distinguish
/// hint-introduced separators from real spaces in the source text.
pub const KANA_HINT_SPACE: char = '\u{200b}';

// =========================================================================
// *kana-hint-mod* (dict-split.lisp:813)
// =========================================================================

/// Sentinel character marking a kana-particle boundary that the
/// romanizer should rewrite (`は → wa`, `へ → e`, …). Inserted by the
/// hint system and consumed by [`hint_simplify_map`] during
/// romanization.
pub const KANA_HINT_MOD: char = '\u{200c}';

// =========================================================================
// *hint-char-map* (dict-split.lisp:816)
// =========================================================================

/// Plist mapping each [`crate::dict::kani::KaniHintKind`] tag to the
/// sentinel character the hint system splices into a kana string at
/// that tag's position. Looked up by [`super::hint::insert_hints`]
/// (mirrors `(getf *hint-char-map* character-kw)`) and scanned by
/// [`super::hint::strip_hints`] (removes any char appearing as a
/// value here).
///
/// The Lisp `defparameter` is a flat plist
/// `(:space ,*kana-hint-space* :mod ,*kana-hint-mod*)`. The Rust port
/// holds the same key/value pairs as a typed slice of
/// `(KaniHintKind, char)` rather than a heterogeneous plist; both
/// consumers (`insert-hints`, `strip-hints`) only ever scan it
/// pairwise.
pub const HINT_CHAR_MAP: [(KaniHintKind, char); 2] = [
    (KaniHintKind::Space, KANA_HINT_SPACE),
    (KaniHintKind::Mod, KANA_HINT_MOD),
];

// =========================================================================
// *hint-simplify-map* (dict-split.lisp:818-824)
// =========================================================================

/// Ordered (from, to) substitution table consumed by
/// [`super::hint::process_hints`] via
/// [`crate::characters::normalize::simplify_ngrams`]. Folds the hint
/// sentinels back into reader-facing characters:
///
/// - `*kana-hint-space*` → ASCII space `" "`
/// - `*kana-hint-mod*` + `は` → `わ` (and `ハ` → `ワ`)
/// - `*kana-hint-mod*` + `へ` → `え` (and `ヘ` → `エ`)
/// - lone `*kana-hint-mod*` → empty string (drop)
///
/// Order is load-bearing: the 2-char sentinel+kana entries must
/// precede the lone-sentinel entry so `simplify_ngrams`' alternation
/// prefers the longer match at the same starting offset.
///
/// The Lisp `defparameter` builds the value at load time from
/// `*kana-hint-space*` / `*kana-hint-mod*` via `string` / `coerce`.
/// The Rust port mirrors that derivation under `OnceLock` rather than
/// freezing the result, so the table stays tracked against the source
/// character constants.
pub fn hint_simplify_map() -> &'static [(String, &'static str)] {
    static CACHE: OnceLock<Vec<(String, &'static str)>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut map: Vec<(String, &'static str)> = Vec::with_capacity(6);
            map.push((KANA_HINT_SPACE.to_string(), " "));
            map.push(([KANA_HINT_MOD, 'は'].iter().collect(), "わ"));
            map.push(([KANA_HINT_MOD, 'ハ'].iter().collect(), "ワ"));
            map.push(([KANA_HINT_MOD, 'へ'].iter().collect(), "え"));
            map.push(([KANA_HINT_MOD, 'ヘ'].iter().collect(), "エ"));
            map.push((KANA_HINT_MOD.to_string(), ""));
            map
        })
        .as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EASY_HINTS has exactly 431 entries, matching the
    /// `def-easy-hint` callsite count in `dict-split.lisp`.
    #[test]
    fn easy_hints_count_matches_upstream() {
        assert_eq!(EASY_HINTS.len(), 431);
    }

    /// All easy-hint kanji-splits are non-empty (smoke-test against
    /// a regex/parser miss in the data extraction).
    #[test]
    fn easy_hints_no_empty_strings() {
        for e in EASY_HINTS {
            assert!(
                !e.kanji_split.is_empty(),
                "empty kanji_split for seq {}",
                e.seq
            );
        }
    }

    // (`EASY_HINTS_SEQS` ↔ `EASY_HINTS` agreement is now structural —
    // `easy_hints_seqs()` derives directly from `EASY_HINTS` via
    // OnceLock per CONVENTIONS §5.2, so they cannot disagree.)

    #[test]
    fn search_chars_finds_first() {
        assert_eq!(search_chars("は", "こんにちはまた", false), Some(4));
    }

    #[test]
    fn search_chars_from_end_finds_last() {
        assert_eq!(search_chars("は", "はは", true), Some(1));
    }

    #[test]
    fn search_chars_substring_multi_char() {
        assert_eq!(search_chars("では", "それではない", true), Some(2));
    }

    #[test]
    fn search_chars_missing_returns_none() {
        assert_eq!(search_chars("は", "こんに", false), None);
    }

    #[test]
    fn ends_with_char_basic() {
        assert!(ends_with_char("こんにちは", 'は'));
        assert!(!ends_with_char("こんにちは!", 'は'));
        assert!(!ends_with_char("", 'は'));
    }

    #[test]
    fn safe_hint_drops_negative() {
        assert_eq!(safe_hint(KaniHintKind::Mod, -1), None);
        assert_eq!(safe_hint(KaniHintKind::Mod, 0), Some((KaniHintKind::Mod, 0)));
        assert_eq!(safe_hint(KaniHintKind::Space, 5), Some((KaniHintKind::Space, 5)));
    }

    /// Pin the build output against the introspected upstream value —
    /// catches drift in the source character constants.
    #[test]
    fn hint_simplify_map_matches_introspected_value() {
        let map = hint_simplify_map();
        assert_eq!(map.len(), 6);
        assert_eq!(map[0], ("\u{200b}".to_string(), " "));
        assert_eq!(map[1], ("\u{200c}は".to_string(), "わ"));
        assert_eq!(map[2], ("\u{200c}ハ".to_string(), "ワ"));
        assert_eq!(map[3], ("\u{200c}へ".to_string(), "え"));
        assert_eq!(map[4], ("\u{200c}ヘ".to_string(), "エ"));
        assert_eq!(map[5], ("\u{200c}".to_string(), ""));
    }
}
