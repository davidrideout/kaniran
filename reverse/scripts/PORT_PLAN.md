# Port plan — 857 symbols in 796 waves (8 mutual-recursion groups covering 69 symbols)
_skipped packages: ichiran/maintenance, ichiran/test_

   1. `ichiran/characters:*half-width-kana*`  — global, characters.lisp:106  *[ported]*
   2. `ichiran/characters:*abnormal-chars*`  — global, characters.lisp:109  *[ported]*
   3. `ichiran/characters:*iteration-characters*`  — global, characters.lisp:5  *[ported]*
   4. `ichiran/characters:*kana-characters*`  — global, characters.lisp:11  *[ported]*
   5. `ichiran/characters:*modifier-characters*`  — global, characters.lisp:7  *[ported]*
   6. `ichiran/characters:*sokuon-characters*`  — global, characters.lisp:3  *[ported]*
   7. `ichiran/characters:*all-characters*`  — global, characters.lisp:32  *[ported]*
   8. `ichiran/characters:*decimal-point-regex*`  — global, characters.lisp:129  *[ported]*
   9. `ichiran/characters:*digit-regex*`  — global, characters.lisp:128  *[ported]*
  10. `ichiran/characters:*num-word-regex*`  — global, characters.lisp:126  *[ported]*
  11. `ichiran/characters:*word-regex*`  — global, characters.lisp:127  *[ported]*
  12. `ichiran/characters:*basic-split-regex*`  — global, characters.lisp:131  *[ported]*
  13. `ichiran/characters:*char-class-hash*`  — global, characters.lisp:37  *[ported]*
  14. `ichiran/characters:*char-class-regex-mapping*`  — global, characters.lisp:136  *[ported]*
  15. `ichiran/characters:*char-scanners*`  — global, characters.lisp:151  *[ported]*
  16. `ichiran/characters:*char-scanners-inner*`  — global, characters.lisp:155  *[ported]*
  17. `ichiran/characters:*dakuten-hash*`  — global, characters.lisp:0  *[ported]*
  18. `ichiran/characters:*handakuten-hash*`  — global, characters.lisp:0  *[ported]*
  19. `ichiran/characters:dakuten-join`  — fn, characters.lisp:100  *[ported]*
  20. `ichiran/characters:*dakuten-join*`  — global, characters.lisp:103  *[ported]*
  21. `ichiran/characters:*full-width-kana*`  — global, characters.lisp:107  *[ported]*
  22. `ichiran/characters:*hiragana-regex*`  — global, characters.lisp:120  *[ported]*
  23. `ichiran/characters:*kanji-char-regex*`  — global, characters.lisp:122  *[ported]*
  24. `ichiran/characters:*kanji-regex*`  — global, characters.lisp:121  *[ported]*
  25. `ichiran/characters:*katakana-regex*`  — global, characters.lisp:118  *[ported]*
  26. `ichiran/characters:*katakana-uniq-regex*`  — global, characters.lisp:119  *[ported]*
  27. `ichiran/characters:*nonword-regex*`  — global, characters.lisp:124  *[ported]*
  28. `ichiran/characters:*normal-chars*`  — global, characters.lisp:114  *[ported]*
  29. `ichiran/characters:*numeric-regex*`  — global, characters.lisp:125  *[ported]*
  30. `ichiran/characters:*punctuation-marks*`  — global, characters.lisp:85  *[ported]*
  31. `ichiran/characters:*undakuten-hash*`  — global, characters.lisp:0  *[ported]*
  32. `ichiran/characters:to-normal-char`  — fn, characters.lisp:242  *[ported]*
  33. `ichiran/characters:as-hiragana`  — fn, characters.lisp:282  *[ported]*
  34. `ichiran/characters:as-katakana`  — fn, characters.lisp:292  *[ported]*
  35. `ichiran/characters:split-by-regex`  — fn, characters.lisp:263  *[ported]*
  36. `ichiran/characters:test-word`  — fn, characters.lisp:187  *[ported]*
  37. `ichiran/characters:basic-split`  — fn, characters.lisp:267  *[ported]*
  38. `ichiran/characters:char-class`  — type, characters.lisp:147  *[ported]*
  39. `ichiran/characters:collect-char-class`  — fn, characters.lisp:201  *[ported]*
  40. `ichiran/characters:consecutive-char-groups`  — fn, characters.lisp:300  *[ported]*
  41. `ichiran/characters:count-char-class`  — fn, characters.lisp:194  *[ported]*
  42. `ichiran/characters:destem`  — fn, characters.lisp:340  *[ported]*
  43. `ichiran/characters:geminate`  — fn, characters.lisp:336  *[ported]*  *[extracted: counter_2026_05_08]*
  44. `ichiran/characters:get-char-class`  — fn, characters.lisp:52  *[ported]*
  45. `ichiran/characters:hash-from-list`  — macro, characters.lisp:64  *[ported]*
  46. `ichiran/characters:join`  — fn, characters.lisp:371  *[ported]*
  47. `ichiran/characters:kanji-cross-match`  — fn, characters.lisp:222  *[ported]*
  48. `ichiran/characters:kanji-mask`  — fn, characters.lisp:212  *[ported]*
  49. `ichiran/characters:kanji-regex`  — fn, characters.lisp:215  *[ported]*
  50. `ichiran/characters:kanji-match`  — fn, characters.lisp:220  *[ported]*
  51. `ichiran/characters:kanji-prefix`  — fn, characters.lisp:306  *[ported]*
  52. `ichiran/characters:long-vowel-modifier-p`  — fn, characters.lisp:54  *[ported]*
  53. `ichiran/characters:match-diff`  — fn, characters.lisp:347  *[ported]*
  54. `ichiran/characters:mora-length`  — fn, characters.lisp:275  *[ported]*
  55. `ichiran/characters:simplify-ngrams`  — fn, characters.lisp:230  *[ported]*
  56. `ichiran/characters:normalize`  — fn, characters.lisp:247  *[ported]*
  57. `ichiran/characters:rendaku`  — fn, characters.lisp:320  *[ported]*  *[extracted: counter_2026_05_08]*
  58. `ichiran/characters:safe-subseq`  — fn, characters.lisp:371  *[ported]*
  59. `ichiran/characters:sequential-kanji-positions`  — fn, characters.lisp:207  *[ported]*
  60. `ichiran/characters:unrendaku`  — fn, characters.lisp:308  *[ported]*
  61. `ichiran/characters:voice-char`  — fn, characters.lisp:91  *[ported]*
  62. `ichiran/cli:print-error`  — fn, cli.lisp:37  *[skip — "CLI-only stderr/debugger glue; Rust uses eprintln!/anyhow/panic-hook. Belongs in a future kaniran-cli crate]*
  63. `ichiran/cli:setup-debugger`  — fn, cli.lisp:95  *[skip — "CLI-only stderr/debugger glue; Rust uses eprintln!/anyhow/panic-hook. Belongs in a future kaniran-cli crate]*
  64. `ichiran/conn:cache-name`  — gf, conn.lisp:0
  65. `ichiran/conn:cache`  — class, conn.lisp:96  *[skip — "Class with one cached value]*
  66. `ichiran/conn:all-caches`  — fn, conn.lisp:110  *[skip — Class-slot registry pattern doesn't translate. Replaced in Rust by per-cache OnceLock + DI when the DB layer lands; no 1:1 counterpart.]*
  67. `ichiran/conn:get-cache`  — fn, conn.lisp:113  *[skip — Looks up a cache instance from the class-side hash by name. Subsumed by typed-field access on Ctx; no name->instance dispatch.]*
  68. `ichiran/conn:init-cache`  — gf, conn.lisp:0  *[skip — "Generic-function dispatch on a cache name keyword. Per-cache builders become methods on Ctx]*
  69. `ichiran/conn:ensure`  — gf, conn.lisp:0  *[skip — Generic-function lazy-init on a cache name. Per-cache lazy access becomes a method on Ctx over its OnceCell field.]*
  70. `ichiran/conn:reset-cache`  — gf, conn.lisp:0  *[skip — Generic-function force-rebuild of a named cache. Per-cache reset becomes a method on Ctx; no name->instance dispatch.]*
  71. `ichiran/conn:init-all-caches`  — fn, conn.lisp:144  *[skip — Class-slot registry pattern doesn't translate. Replaced in Rust by per-cache OnceLock + DI when the DB layer lands; no 1:1 counterpart.]*
  72. `ichiran/conn:*conn-var-cache*`  — global, conn.lisp:41  *[skip — Cache mapping (var . spec) -> value for the per-connection rebinding. Subsumed by per-Ctx field ownership.]*
  73. `ichiran/conn:*test-var*`  — global, conn.lisp:0  *[skip — Test fixture for the def-conn-var rebinding system; obsolete with per-Ctx ownership.]*
  74. `ichiran/conn:*connection*`  — global, settings.lisp:3  *[skip — Active connection spec global. State lives on Ctx::pool; constructed via Ctx::from_url or Ctx::from_env.]*
  75. `ichiran/dict:*counter-accepts*`  — global, dict-counters.lisp:217  *[ported]*
  76. `ichiran/dict:*counter-foreign*`  — global, dict-counters.lisp:219  *[ported]*
  77. `ichiran/dict:*counter-suffixes*`  — global, dict-counters.lisp:213  *[ported]*
  78. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:compound-text`  — class, dict.lisp:608  *[ported]*
        - `ichiran/dict:score-base`  — gf, dict.lisp:0  *[ported]*
  79. `ichiran/dict:entry`  — dao, dict.lisp:26  *[ported]*
  80. `ichiran/dict:simple-text`  — class, dict.lisp:69  *[ported]*
  81. `ichiran/dict:kana-text`  — dao, dict.lisp:128  *[ported]*
  82. `ichiran/dict:kanji-text`  — dao, dict.lisp:86  *[ported]*
  83. `ichiran/dict:proxy-text`  — class, dict.lisp:550  *[ported]*
  84. `ichiran/dict:conjugation`  — dao, dict.lisp:238  *[ported]*
  85. `ichiran/dict:restricted-readings`  — dao, dict.lisp:221  *[ported]*
  86. `ichiran/dict:sense`  — dao, dict.lisp:166  *[ported]*
  87. `ichiran/dict:sense-prop`  — dao, dict.lisp:197  *[ported]*
  88. `ichiran/dict:conj-source-reading`  — dao, dict.lisp:309  *[ported]*
  89. `ichiran/dict:gloss`  — dao, dict.lisp:178  *[ported]*
  90. `ichiran/dict:text`  — gf, dict-counters.lisp:0  *[ported]*
  91. **CYCLE (4 symbols — port together)**
        - `ichiran/dict:common`  — gf, dict-counters.lisp:0  *[ported]*  *[extracted: tatoeba]*  *[audited 3180/3180]*
        - `ichiran/dict:counter-text`  — class, dict-counters.lisp:9  *[ported]*
        - `ichiran/dict:seq`  — gf, dict-counters.lisp:0  *[ported]*  *[extracted: tatoeba]*  *[audited 3795/3795]*
        - `ichiran/dict:source`  — gf, dict-counters.lisp:0  *[ported]*  *[extracted: tatoeba]*  *[audited 27/27]*
  92. `ichiran/dict:counter-age`  — class, dict-counters.lisp:757  *[ported]*
  93. `ichiran/dict:counter-days-kun`  — class, dict-counters.lisp:686  *[ported]*
  94. `ichiran/dict:counter-days-on`  — class, dict-counters.lisp:709  *[ported]*
  95. `ichiran/dict:counter-halfhour`  — class, dict-counters.lisp:391  *[ported]*
  96. `ichiran/dict:counter-hifumi`  — class, dict-counters.lisp:518  *[ported]*
  97. `ichiran/dict:counter-months`  — class, dict-counters.lisp:721  *[ported]*
  98. `ichiran/dict:counter-people`  — class, dict-counters.lisp:735  *[ported]*
  99. `ichiran/dict:counter-tsu`  — class, dict-counters.lisp:497  *[ported]*
 100. `ichiran/dict:counter-wari`  — class, dict-counters.lisp:746  *[ported]*
 101. `ichiran/dict:*special-counters*`  — global, dict-counters.lisp:211  *[ported]*
 102. `ichiran/dict:*extra-counter-ids*`  — global, dict-counters.lisp:310  *[ported]*
 103. `ichiran/dict:*skip-counter-ids*`  — global, dict-counters.lisp:315  *[ported]*
 104. `ichiran/dict:get-counter-ids`  — fn, dict-counters.lisp:285  *[ported]*
 105. `ichiran/dict:get-counter-stags`  — fn, dict-counters.lisp:292  *[ported]*
 106. `ichiran/dict:ord`  — gf, dict-counters.lisp:0  *[ported]*
 107. `ichiran/dict:get-counter-readings`  — fn, dict-counters.lisp:335  *[ported]*
 108. `ichiran/dict:no-conj-data`  — fn, dict.lisp:337  *[ported]*
 109. `ichiran/dict:*suffix-cache*`  — global, dict-grammar.lisp:0  *[wip — empty-map stub: populated by wave 127 init-suffixes via init-suffix-hashtables + load-kf + every def-simple-suffix callsite. Replace with proper init when wave 127 lands.]*
 110. `ichiran/dict:*suffix-class*`  — global, dict-grammar.lisp:0  *[wip — empty-map stub: populated by wave 127 init-suffixes via init-suffix-hashtables + load-kf + every def-simple-suffix callsite. Replace with proper init when wave 127 lands.]*
 111. **CYCLE (4 symbols — port together)**
        - `ichiran/conn:*conn-vars*`  — global, conn.lisp:39  *[skip — Registry of per-connection-rebound globals. Unneeded once each Ctx owns its caches directly.]*
        - `ichiran/dict:*counter-cache*`  — global, dict-counters.lisp:0  *[ported]*
        - `ichiran/dict:*is-arch-cache*`  — global, dict.lisp:0  *[ported]*
        - `ichiran/dict:*no-conj-data*`  — global, dict.lisp:0  *[ported]*
 112. `ichiran/conn:*connections*`  — global, settings.lisp:5  *[skip — Alist of secondary connection specs. Replaced by call-site Ctx::from_url(...) per database; no global registry.]*
 113. `ichiran/conn:get-spec`  — fn, conn.lisp:25  *[skip — Lisp dbid-dispatch (nil/list/keyword → connection spec) doesn't translate. Connection registry will be handled via the Rust config crate when the DB layer lands.]*
 114. `ichiran/conn:switch-conn-vars`  — fn, conn.lisp:65  *[skip — Per-connection variable rebinding from *conn-var-cache*. Rust has no dynamic-variable shadowing; replaced by per-Database struct ownership of caches when the DB layer lands. Same family as all-caches / get-spec.]*
 115. `ichiran/dict:init-suffix-hashtables`  — fn, dict-grammar.lisp:6  *[skip — Empty-hashtable initializer for *suffix-cache* / *suffix-class* def-conn-vars. Rust replacement is OnceLock<HashMap> populated on first read; no standalone init verb survives.]*
 116. `ichiran/dict:*init-suffixes-lock*`  — global, dict-grammar.lisp:163  *[skip — SBCL mutex guarding init-suffixes-thread's populator and powering init-suffixes-running-p. Subsumed by OnceLock::get_or_init's built-in once-only synchronization on *suffix-cache* / *suffix-class*; no standalone mutex survives.]*
 117. `ichiran/dict:init-suffixes-running-p`  — fn, dict-grammar.lisp:165  *[skip — Loader-busy predicate over a one-shot init thread + def-conn-var cache. Rust replacement is OnceLock::get().is_some() or eager startup init; the verb has nowhere to live.]*
 118. `ichiran/dict:find-word-seq`  — fn, dict-grammar.lisp:73  *[ported]*
 119. `ichiran/dict:conj-prop`  — dao, dict.lisp:262  *[ported]*
 120. `ichiran/dict:id`  — gf, dict.lisp:0  *[skip — Slot-reader gf with no polymorphic callsites; every (id X) / (conj-id X) site has a locally-known DAO type. Each Rust DAO struct exposes pub id: i32 / pub conj_id: i32 directly per CONVENTIONS §4.7.]*
 121. `ichiran/dict:find-word-conj-of`  — fn, dict-grammar.lisp:77  *[ported]*
 122. `ichiran/dict:get-kana-form`  — fn, dict-grammar.lisp:36  *[ported]*
 123. `ichiran/dict:conj-data`  — struct, dict.lisp:327  *[ported]*
 124. `ichiran/dict:conj-id`  — gf, dict.lisp:0  *[skip — Slot-reader gf with no polymorphic callsites; every (id X) / (conj-id X) site has a locally-known DAO type. Each Rust DAO struct exposes pub id: i32 / pub conj_id: i32 directly per CONVENTIONS §4.7.]*
 125. `ichiran/dict:get-conj-data`  — fn, dict.lisp:340  *[ported]*
 126. `ichiran/dict:*weak-conj-forms*`  — global, dict-errata.lisp:1316  *[ported]*
 127. `ichiran/dict:*skip-conj-forms*`  — global, dict-errata.lisp:1310  *[ported]*
 128. `ichiran/dict:test-conj-prop`  — fn, dict-errata.lisp:1336  *[ported]*
 129. `ichiran/dict:skip-by-conj-data`  — fn, dict-errata.lisp:1336  *[ported]*  *[extracted: tatoeba]*  *[audited 33168/33168]*
 130. `ichiran/dict:get-kana-forms-conj-data-filter`  — fn, dict-grammar.lisp:10  *[ported]*  *[extracted: init-suffixes]*
 131. `ichiran/dict:get-kana-forms*`  — fn, dict-grammar.lisp:17  *[ported]*  *[extracted: init-suffixes]*
 132. `ichiran/dict:get-kana-forms`  — fn, dict-grammar.lisp:32  *[ported]*  *[extracted: init-suffixes]*
 133. `ichiran/dict:init-suffixes-thread`  — fn, dict-grammar.lisp:169  *[ported]*
 134. `ichiran/dict:init-suffixes`  — fn, dict-grammar.lisp:332  *[skip — Subsumed by KaniranContext eager construction; init-suffixes-thread (wave 126) is the actual populator. Same prior-art as init-suffixes-running-p.]*
 135. `ichiran/cli:build`  — fn, cli.lisp:102  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 136. `ichiran/cli:print-romanize-info`  — fn, cli.lisp:44  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 137. `ichiran/cli:unknown-option`  — fn, cli.lisp:33  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 138. `ichiran/conn:*is-dynamic-connection*`  — global, conn.lisp:14  *[skip — "Boolean marking 'connection came from env]*
 139. `ichiran/conn:*connection-env-var*`  — global, conn.lisp:13  *[ported]*
 140. `ichiran/conn:get-ichiran-connection-env`  — fn, conn.lisp:154  *[ported]*
 141. `ichiran/conn:load-connection-from-env`  — fn, conn.lisp:166  *[skip — "Side-effects-on-globals semantics (set *connection*]*
 142. `ichiran/dict:to-json`  — gf, writer.lisp:0
 143. `ichiran/dict:true-text`  — gf, dict.lisp:0  *[ported]*
 144. `ichiran/dict:word-info`  — class, dict.lisp:1245  *[ported]*
 145. `ichiran/dict:process-word-info`  — fn, dict.lisp:1417  *[ported]*
 146. `ichiran/dict:synergy`  — struct, dict-grammar.lisp:713  *[ported]*
 147. **CYCLE (4 symbols — port together)**
        - `ichiran/dict:segment`  — struct, dict.lisp:674  *[ported]*
        - `ichiran/dict:segment-list`  — struct, dict.lisp:1038  *[ported]*
        - `ichiran/dict:top-array`  — class, dict.lisp:1140  *[ported]*
        - `ichiran/dict:top-array-item`  — struct, dict.lisp:1138  *[ported]*
 148. `ichiran/dict:*segment-score-cutoff*`  — global, dict.lisp:1351  *[ported]*
 149. `ichiran/dict:*disable-hints*`  — global, dict.lisp:78  *[ported]*
 150. `ichiran/dict:*kana-hint-space*`  — global, dict-split.lisp:814  *[ported]*
 151. `ichiran/dict:query-parents-kanji`  — fn, dict.lisp:400  *[extracted: tatoeba]*
 152. `ichiran/dict:best-kana-conj`  — fn, dict.lisp:428  *[extracted: tatoeba]*
 153. `ichiran/dict:get-digit`  — fn, dict-counters.lisp:94  *[ported]*  *[extracted: counter_2026_05_08]*  *[audited 193/193]*
 154. `ichiran/numbers:*digit-to-kana*`  — global, numbers.lisp:25  *[ported]*
 155. `ichiran/numbers:*power-to-kana*`  — global, numbers.lisp:28  *[ported]*
 156. `ichiran/dict:counter-join`  — gf, dict-counters.lisp:0  *[ported]*  *[extracted: counter_2026_05_08]*
 157. `ichiran/dict:*hint-map*`  — global, dict-split.lisp:850
 158. `ichiran/dict:word-conj-data`  — gf, dict.lisp:0
 159. `ichiran/dict:get-hint`  — fn, dict-split.lisp:968
 160. `ichiran/dict:get-kanji-kana-old`  — fn, dict.lisp:115  *[extracted: tatoeba]*
 161. `ichiran/numbers:*char-number-class*`  — global, numbers.lisp:9  *[ported]*
 162. `ichiran/numbers:*char-number-class-hash*`  — global, numbers.lisp:18  *[ported]*
 163. `ichiran/numbers:num-sandhi`  — gf, numbers.lisp:0  *[ported]*
 164. `ichiran/numbers:group-to-kana`  — fn, numbers.lisp:117  *[ported]*  *[extracted: counter_2026_05_08]*
 165. `ichiran/numbers:*digit-kanji-default*`  — global, numbers.lisp:3  *[ported]*
 166. `ichiran/numbers:*power-kanji*`  — global, numbers.lisp:7  *[ported]*
 167. `ichiran/numbers:number-to-kanji`  — fn, numbers.lisp:35  *[ported]*  *[extracted: counter_2026_05_08]*
 168. `ichiran/numbers:number-to-kana`  — fn, numbers.lisp:125  *[ported]*  *[extracted: counter_2026_05_08]*
 169. `ichiran/dict:get-kana`  — gf, dict.lisp:0  *[extracted: tatoeba]*
 170. `ichiran/dict:get-text`  — gf, dict.lisp:0  *[extracted: tatoeba]*
 171. `ichiran/dict:ordinal-str`  — fn, dict-counters.lisp:38  *[ported]*  *[extracted: counter_2026_05_08]*
 172. `ichiran/dict:value-string`  — gf, dict-counters.lisp:0  *[extracted: counter_2026_05_08]*
 173. `ichiran/dict:word-type`  — gf, dict.lisp:0
 174. `ichiran/dict:word-info-from-segment`  — fn, dict.lisp:1327
 175. `ichiran/dict:word-info-from-segment-list`  — fn, dict.lisp:1353
 176. `ichiran/dict:fill-segment-path`  — fn, dict.lisp:1390
 177. `ichiran/dict:split-1010105`  — fn, dict-split.lisp:771
 178. `ichiran/dict:split-1567610`  — fn, dict-split.lisp:771
 179. `ichiran/dict:split-1675330`  — fn, dict-split.lisp:771
 180. `ichiran/dict:split-2841254`  — fn, dict-split.lisp:771
 181. `ichiran/dict:split-dakara`  — fn, dict-split.lisp:771
 182. `ichiran/dict:split-deha`  — fn, dict-split.lisp:771
 183. `ichiran/dict:split-dokoroka`  — fn, dict-split.lisp:771
 184. `ichiran/dict:split-hitorashii`  — fn, dict-split.lisp:771
 185. `ichiran/dict:split-honno`  — fn, dict-split.lisp:771
 186. `ichiran/dict:split-kanatte`  — fn, dict-split.lisp:771
 187. `ichiran/dict:split-naito`  — fn, dict-split.lisp:771
 188. `ichiran/dict:split-omise`  — fn, dict-split.lisp:771
 189. `ichiran/dict:split-toha`  — fn, dict-split.lisp:771
 190. `ichiran/dict:split-tokorode`  — fn, dict-split.lisp:771
 191. `ichiran/dict:split-tokorodewa`  — fn, dict-split.lisp:771
 192. `ichiran/dict:split-tokoroe`  — fn, dict-split.lisp:771
 193. `ichiran/dict:split-tokoroga`  — fn, dict-split.lisp:771
 194. `ichiran/dict:split-tokorowo`  — fn, dict-split.lisp:771
 195. `ichiran/dict:*segsplit-map*`  — global, dict-split.lisp:704
 196. `ichiran/dict:split-1000430`  — fn, dict-split.lisp:505
 197. `ichiran/dict:split-1002970`  — fn, dict-split.lisp:492
 198. `ichiran/dict:split-1005600`  — fn, dict-split.lisp:498
 199. `ichiran/dict:split-1006280`  — fn, dict-split.lisp:669
 200. `ichiran/dict:split-1006880`  — fn, dict-split.lisp:727
 201. `ichiran/dict:split-1008030`  — fn, dict-split.lisp:645
 202. `ichiran/dict:split-1207840`  — fn, dict-split.lisp:711
 203. `ichiran/dict:split-1221530`  — fn, dict-split.lisp:611
 204. `ichiran/dict:split-1221680`  — fn, dict-split.lisp:521
 205. `ichiran/dict:split-1314600`  — fn, dict-split.lisp:512
 206. `ichiran/dict:split-1314770`  — fn, dict-split.lisp:640
 207. `ichiran/dict:split-1315860`  — fn, dict-split.lisp:535
 208. `ichiran/dict:split-1322540`  — fn, dict-split.lisp:517
 209. `ichiran/dict:split-1322560`  — fn, dict-split.lisp:719
 210. `ichiran/dict:split-1327220`  — fn, dict-split.lisp:424
 211. `ichiran/dict:split-1327230`  — fn, dict-split.lisp:429
 212. `ichiran/dict:split-1349300`  — fn, dict-split.lisp:608
 213. `ichiran/dict:split-1362970`  — fn, dict-split.lisp:759
 214. `ichiran/dict:split-1474200`  — fn, dict-split.lisp:546
 215. `ichiran/dict:split-1502500`  — fn, dict-split.lisp:487
 216. `ichiran/dict:split-1508380`  — fn, dict-split.lisp:478
 217. `ichiran/dict:split-1532270`  — fn, dict-split.lisp:685
 218. `ichiran/dict:split-1538340`  — fn, dict-split.lisp:526
 219. `ichiran/dict:split-1551500`  — fn, dict-split.lisp:631
 220. `ichiran/dict:split-1579130`  — fn, dict-split.lisp:559
 221. `ichiran/dict:split-1581550`  — fn, dict-split.lisp:650
 222. `ichiran/dict:split-1591050`  — fn, dict-split.lisp:571
 223. `ichiran/dict:split-1591980`  — fn, dict-split.lisp:625
 224. `ichiran/dict:split-1597740`  — fn, dict-split.lisp:645
 225. `ichiran/dict:split-1601010`  — fn, dict-split.lisp:732
 226. `ichiran/dict:split-1601080`  — fn, dict-split.lisp:658
 227. `ichiran/dict:split-1602740`  — fn, dict-split.lisp:605
 228. `ichiran/dict:split-1606530`  — fn, dict-split.lisp:676
 229. `ichiran/dict:split-1606800`  — fn, dict-split.lisp:706
 230. `ichiran/dict:split-1612640`  — fn, dict-split.lisp:509
 231. `ichiran/dict:split-1774820`  — fn, dict-split.lisp:756
 232. `ichiran/dict:split-1854750`  — fn, dict-split.lisp:596
 233. `ichiran/dict:split-1855670`  — fn, dict-split.lisp:742
 234. `ichiran/dict:split-1863230`  — fn, dict-split.lisp:698
 235. `ichiran/dict:split-1881690`  — fn, dict-split.lisp:734
 236. `ichiran/dict:optprefix`  — fn, dict-split.lisp:580
 237. `ichiran/dict:split-1894260`  — fn, dict-split.lisp:586
 238. `ichiran/dict:split-2002270`  — fn, dict-split.lisp:633
 239. `ichiran/dict:split-2007500`  — fn, dict-split.lisp:681
 240. `ichiran/dict:split-2009290`  — fn, dict-split.lisp:483
 241. `ichiran/dict:split-2016840`  — fn, dict-split.lisp:502
 242. `ichiran/dict:split-2026650`  — fn, dict-split.lisp:601
 243. `ichiran/dict:split-2083990`  — fn, dict-split.lisp:468
 244. `ichiran/dict:split-2088480`  — fn, dict-split.lisp:438
 245. `ichiran/dict:split-2109610`  — fn, dict-split.lisp:715
 246. `ichiran/dict:split-2133750`  — fn, dict-split.lisp:691
 247. `ichiran/dict:split-2272780`  — fn, dict-split.lisp:616
 248. `ichiran/dict:split-2276360`  — fn, dict-split.lisp:554
 249. `ichiran/dict:split-2433760`  — fn, dict-split.lisp:432
 250. `ichiran/dict:split-2526850`  — fn, dict-split.lisp:597
 251. `ichiran/dict:split-2529050`  — fn, dict-split.lisp:662
 252. `ichiran/dict:split-2666360`  — fn, dict-split.lisp:446
 253. `ichiran/dict:split-2668400`  — fn, dict-split.lisp:564
 254. `ichiran/dict:split-2724560`  — fn, dict-split.lisp:442
 255. `ichiran/dict:split-2757500`  — fn, dict-split.lisp:531
 256. `ichiran/dict:split-2757540`  — fn, dict-split.lisp:673
 257. `ichiran/dict:split-2762260`  — fn, dict-split.lisp:474
 258. `ichiran/dict:split-2771940`  — fn, dict-split.lisp:457
 259. `ichiran/dict:split-2834051`  — fn, dict-split.lisp:702
 260. `ichiran/dict:split-2834732`  — fn, dict-split.lisp:740
 261. `ichiran/dict:split-2835890`  — fn, dict-split.lisp:577
 262. `ichiran/dict:split-2846470`  — fn, dict-split.lisp:621
 263. `ichiran/dict:split-2855921`  — fn, dict-split.lisp:748
 264. `ichiran/dict:split-de-1004800`  — fn, dict-split.lisp:104
 265. `ichiran/dict:split-de-1006840`  — fn, dict-split.lisp:106
 266. `ichiran/dict:split-de-1163700`  — fn, dict-split.lisp:102
 267. `ichiran/dict:split-de-1189420`  — fn, dict-split.lisp:111
 268. `ichiran/dict:split-de-1245390`  — fn, dict-split.lisp:108
 269. `ichiran/dict:split-de-1270210`  — fn, dict-split.lisp:140
 270. `ichiran/dict:split-de-1272220`  — fn, dict-split.lisp:112
 271. `ichiran/dict:split-de-1311360`  — fn, dict-split.lisp:113
 272. `ichiran/dict:split-de-1343110`  — fn, dict-split.lisp:139
 273. `ichiran/dict:split-de-1368500`  — fn, dict-split.lisp:114
 274. `ichiran/dict:split-de-1395670`  — fn, dict-split.lisp:115
 275. `ichiran/dict:split-de-1417790`  — fn, dict-split.lisp:116
 276. `ichiran/dict:split-de-1454270`  — fn, dict-split.lisp:117
 277. `ichiran/dict:split-de-1479100`  — fn, dict-split.lisp:119
 278. `ichiran/dict:split-de-1510140`  — fn, dict-split.lisp:120
 279. `ichiran/dict:split-de-1518550`  — fn, dict-split.lisp:121
 280. `ichiran/dict:split-de-1530610`  — fn, dict-split.lisp:107
 281. `ichiran/dict:split-de-1531420`  — fn, dict-split.lisp:122
 282. `ichiran/dict:split-de-1597400`  — fn, dict-split.lisp:123
 283. `ichiran/dict:split-de-1611020`  — fn, dict-split.lisp:102
 284. `ichiran/dict:split-de-1679990`  — fn, dict-split.lisp:124
 285. `ichiran/dict:split-de-1682060`  — fn, dict-split.lisp:126
 286. `ichiran/dict:split-de-1736650`  — fn, dict-split.lisp:127
 287. `ichiran/dict:split-de-1865020`  — fn, dict-split.lisp:128
 288. `ichiran/dict:split-de-1878880`  — fn, dict-split.lisp:129
 289. `ichiran/dict:split-de-2126220`  — fn, dict-split.lisp:130
 290. `ichiran/dict:split-de-2136520`  — fn, dict-split.lisp:131
 291. `ichiran/dict:split-de-2513590`  — fn, dict-split.lisp:133
 292. `ichiran/dict:split-de-2719270`  — fn, dict-split.lisp:109
 293. `ichiran/dict:split-de-2771850`  — fn, dict-split.lisp:135
 294. `ichiran/dict:split-de-2810720`  — fn, dict-split.lisp:105
 295. `ichiran/dict:split-de-2810800`  — fn, dict-split.lisp:136
 296. `ichiran/dict:split-degozaimasu`  — fn, dict-split.lisp:140
 297. `ichiran/dict:split-desura`  — fn, dict-split.lisp:382
 298. `ichiran/dict:split-do-2142680`  — fn, dict-split.lisp:190
 299. `ichiran/dict:split-do-2142710`  — fn, dict-split.lisp:189
 300. `ichiran/dict:split-do-2523480`  — fn, dict-split.lisp:190
 301. `ichiran/dict:split-do-2803190`  — fn, dict-split.lisp:189
 302. `ichiran/dict:split-dogatsukeru`  — fn, dict-split.lisp:276
 303. `ichiran/dict:split-gotoni`  — fn, dict-split.lisp:387
 304. `ichiran/dict:split-hairikomeru`  — fn, dict-split.lisp:340
 305. `ichiran/dict:split-hajiketobu`  — fn, dict-split.lisp:328
 306. `ichiran/dict:split-hajikidasu`  — fn, dict-split.lisp:368
 307. `ichiran/dict:split-hayaimonode`  — fn, dict-split.lisp:267
 308. `ichiran/dict:split-hisshininatte`  — fn, dict-split.lisp:348
 309. `ichiran/dict:split-hitotachi`  — fn, dict-split.lisp:375
 310. `ichiran/dict:split-jan`  — fn, dict-split.lisp:454
 311. `ichiran/dict:split-janai`  — fn, dict-split.lisp:449
 312. `ichiran/dict:split-janaika`  — fn, dict-split.lisp:281
 313. `ichiran/dict:split-kaasan`  — fn, dict-split.lisp:285
 314. `ichiran/dict:split-kaisasae`  — fn, dict-split.lisp:399
 315. `ichiran/dict:split-katawonaraberu`  — fn, dict-split.lisp:305
 316. `ichiran/dict:split-kawaribae`  — fn, dict-split.lisp:258
 317. `ichiran/dict:split-kimatte`  — fn, dict-split.lisp:314
 318. `ichiran/dict:split-kinosei`  — fn, dict-split.lisp:295
 319. `ichiran/dict:split-kotonisuru`  — fn, dict-split.lisp:360
 320. `ichiran/dict:split-motteiku`  — fn, dict-split.lisp:333
 321. `ichiran/dict:split-moushiwakenasasou`  — fn, dict-split.lisp:310
 322. `ichiran/dict:split-nakunaru`  — fn, dict-split.lisp:237
 323. `ichiran/dict:split-nakunaru2`  — fn, dict-split.lisp:244
 324. `ichiran/dict:split-nanimokamo`  — fn, dict-split.lisp:301
 325. `ichiran/dict:split-nantokanaru`  — fn, dict-split.lisp:323
 326. `ichiran/dict:split-nara`  — fn, dict-split.lisp:464
 327. `ichiran/dict:split-nitotte`  — fn, dict-split.lisp:354
 328. `ichiran/dict:split-osagari`  — fn, dict-split.lisp:395
 329. `ichiran/dict:split-osoreiru`  — fn, dict-split.lisp:318
 330. `ichiran/dict:split-shi-1005700`  — fn, dict-split.lisp:209
 331. `ichiran/dict:split-shi-1005830`  — fn, dict-split.lisp:210
 332. `ichiran/dict:split-shi-1157200`  — fn, dict-split.lisp:211
 333. `ichiran/dict:split-shi-1157220`  — fn, dict-split.lisp:212
 334. `ichiran/dict:split-shi-1157230`  — fn, dict-split.lisp:213
 335. `ichiran/dict:split-shi-1157240`  — fn, dict-split.lisp:232
 336. `ichiran/dict:split-shi-1157280`  — fn, dict-split.lisp:214
 337. `ichiran/dict:split-shi-1157310`  — fn, dict-split.lisp:215
 338. `ichiran/dict:split-shi-1304820`  — fn, dict-split.lisp:234
 339. `ichiran/dict:split-shi-1304890`  — fn, dict-split.lisp:216
 340. `ichiran/dict:split-shi-1304960`  — fn, dict-split.lisp:218
 341. `ichiran/dict:split-shi-1305110`  — fn, dict-split.lisp:219
 342. `ichiran/dict:split-shi-1305280`  — fn, dict-split.lisp:221
 343. `ichiran/dict:split-shi-1305290`  — fn, dict-split.lisp:223
 344. `ichiran/dict:split-shi-1594300`  — fn, dict-split.lisp:223
 345. `ichiran/dict:split-shi-1594310`  — fn, dict-split.lisp:225
 346. `ichiran/dict:split-shi-1594460`  — fn, dict-split.lisp:227
 347. `ichiran/dict:split-shi-1594580`  — fn, dict-split.lisp:228
 348. `ichiran/dict:split-shi-2518250`  — fn, dict-split.lisp:231
 349. `ichiran/dict:split-shi-2858937`  — fn, dict-split.lisp:235
 350. `ichiran/dict:split-shinikakaru`  — fn, dict-split.lisp:345
 351. `ichiran/dict:split-souda`  — fn, dict-split.lisp:290
 352. `ichiran/dict:split-soudesu`  — fn, dict-split.lisp:292
 353. `ichiran/dict:split-tegakakaru`  — fn, dict-split.lisp:249
 354. `ichiran/dict:split-toiu`  — fn, dict-split.lisp:404
 355. `ichiran/dict:split-toiukotoda`  — fn, dict-split.lisp:407
 356. `ichiran/dict:split-tonaru`  — fn, dict-split.lisp:419
 357. `ichiran/dict:split-tonattara`  — fn, dict-split.lisp:415
 358. `ichiran/dict:split-toori-1164910`  — fn, dict-split.lisp:174
 359. `ichiran/dict:split-toori-1260990`  — fn, dict-split.lisp:155
 360. `ichiran/dict:split-toori-1368820`  — fn, dict-split.lisp:171
 361. `ichiran/dict:split-toori-1414570`  — fn, dict-split.lisp:157
 362. `ichiran/dict:split-toori-1424950`  — fn, dict-split.lisp:159
 363. `ichiran/dict:split-toori-1424960`  — fn, dict-split.lisp:161
 364. `ichiran/dict:split-toori-1462720`  — fn, dict-split.lisp:179
 365. `ichiran/dict:split-toori-1489800`  — fn, dict-split.lisp:167
 366. `ichiran/dict:split-toori-1523010`  — fn, dict-split.lisp:169
 367. `ichiran/dict:split-toori-1550490`  — fn, dict-split.lisp:172
 368. `ichiran/dict:split-toori-1619440`  — fn, dict-split.lisp:173
 369. `ichiran/dict:split-toori-1808080`  — fn, dict-split.lisp:171
 370. `ichiran/dict:split-toori-1820790`  — fn, dict-split.lisp:165
 371. `ichiran/dict:*split-map*`  — global, dict-split.lisp:5
 372. `ichiran/dict:*copulae*`  — global, dict-errata.lisp:1205
 373. `ichiran/dict:*final-prt*`  — global, dict-errata.lisp:1182
 374. `ichiran/dict:*non-final-prt*`  — global, dict-errata.lisp:1209
 375. `ichiran/dict:*semi-final-prt*`  — global, dict-errata.lisp:1196
 376. `ichiran/dict:*skip-words*`  — global, dict-errata.lisp:1155
 377. `ichiran/dict:apply-score-mod`  — gf, dict.lisp:0
 378. `ichiran/dict:compare-common`  — fn, dict.lisp:1022
 379. `ichiran/dict:get-non-arch-posi`  — fn, dict.lisp:762
 380. `ichiran/dict:get-original-text*`  — fn, dict.lisp:378
 381. `ichiran/dict:get-original-text`  — gf, dict.lisp:0
 382. `ichiran/dict:get-split*`  — fn, dict-split.lisp:67
 383. `ichiran/dict:get-split`  — fn, dict-split.lisp:75
 384. `ichiran/dict:is-arch`  — fn, dict.lisp:760  *[ported]*
 385. `ichiran/dict:*no-kanji-break-penalty*`  — global, dict-errata.lisp:1214
 386. `ichiran/dict:*score-cutoff*`  — global, dict.lisp:1069
 387. `ichiran/dict:parse-suffix-val`  — fn, dict-grammar.lisp:679
 388. `ichiran/dict:make-slice`  — fn, dict.lisp:1010
 389. `ichiran/dict:subseq-slice`  — fn, dict.lisp:1013
 390. `ichiran/dict:get-suffixes`  — fn, dict-grammar.lisp:697
 391. `ichiran/dict:*length-coeff-sequences*`  — global, dict.lisp:686
 392. `ichiran/dict:length-multiplier-coeff`  — fn, dict.lisp:694
 393. `ichiran/dict:nokanji`  — gf, dict-counters.lisp:0
 394. `ichiran/dict:sense-id`  — gf, dict.lisp:0
 395. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:calc-score`  — fn, dict.lisp:775
        - `ichiran/dict:kanji-break-penalty`  — fn, dict.lisp:702
 396. `ichiran/dict:get-segsplit`  — fn, dict-split.lisp:823
 397. `ichiran/dict:expand-segment-list`  — fn, dict.lisp:1180
 398. `ichiran/dict:*gap-penalty*`  — global, dict.lisp:1165
 399. `ichiran/dict:gap-penalty`  — fn, dict.lisp:1169
 400. `ichiran/dict:get-array`  — gf, dict.lisp:0
 401. `ichiran/dict:classify`  — fn, dict-grammar.lisp:1046
 402. `ichiran/dict:filter-in-seq-set`  — fn, dict-grammar.lisp:783
 403. `ichiran/dict:filter-is-conjugation`  — fn, dict-grammar.lisp:797
 404. `ichiran/dict:make-segment-list-from`  — fn, dict-grammar.lisp:733
 405. `ichiran/dict:segfilter-aux-verb`  — fn, dict-grammar.lisp:1099
 406. `ichiran/dict:filter-is-compound-end-text`  — fn, dict-grammar.lisp:820
 407. `ichiran/dict:segfilter-badend`  — fn, dict-grammar.lisp:1114
 408. `ichiran/dict:segfilter-dashi`  — fn, dict-grammar.lisp:1167
 409. `ichiran/dict:segfilter-dekiru`  — fn, dict-grammar.lisp:1175
 410. `ichiran/dict:segfilter-honorific`  — fn, dict-grammar.lisp:1177
 411. `ichiran/dict:filter-is-compound-end`  — fn, dict-grammar.lisp:806
 412. `ichiran/dict:segfilter-janai`  — fn, dict-grammar.lisp:1146
 413. `ichiran/dict:segfilter-mononi`  — fn, dict-grammar.lisp:1177
 414. `ichiran/dict:filter-in-seq-set-simple`  — fn, dict-grammar.lisp:787
 415. `ichiran/dict:segfilter-n`  — fn, dict-grammar.lisp:1106
 416. `ichiran/dict:segfilter-nohayamete`  — fn, dict-grammar.lisp:1151
 417. `ichiran/dict:segfilter-roku`  — fn, dict-grammar.lisp:1129
 418. `ichiran/dict:segfilter-sae`  — fn, dict-grammar.lisp:1141
 419. `ichiran/dict:segfilter-sukiyoki`  — fn, dict-grammar.lisp:1119
 420. `ichiran/dict:segfilter-toomou`  — fn, dict-grammar.lisp:1156
 421. `ichiran/dict:segfilter-totte`  — fn, dict-grammar.lisp:1165
 422. `ichiran/dict:segfilter-tsu-iru`  — fn, dict-grammar.lisp:1101
 423. `ichiran/dict:segfilter-wokarasu`  — fn, dict-grammar.lisp:1112
 424. `ichiran/dict:*segfilter-list*`  — global, dict-grammar.lisp:1024
 425. `ichiran/dict:apply-segfilters`  — fn, dict-grammar.lisp:1177
 426. `ichiran/dict:get-seg-initial`  — fn, dict.lisp:1172
 427. `ichiran/dict:penalty-semi-final`  — fn, dict-grammar.lisp:1022
 428. `ichiran/dict:filter-short-kana`  — fn, dict-grammar.lisp:1008
 429. `ichiran/dict:penalty-short`  — fn, dict-grammar.lisp:1020
 430. `ichiran/dict:*penalty-list*`  — global, dict-grammar.lisp:964
 431. `ichiran/dict:get-penalties`  — fn, dict-grammar.lisp:1030
 432. `ichiran/dict:synergy-kanji-prefix`  — fn, dict-grammar.lisp:940
 433. `ichiran/dict:synergy-na-adjectives`  — fn, dict-grammar.lisp:892
 434. `ichiran/dict:synergy-no-adjectives`  — fn, dict-grammar.lisp:884
 435. `ichiran/dict:synergy-no-da`  — fn, dict-grammar.lisp:871
 436. `ichiran/dict:synergy-no-toori`  — fn, dict-grammar.lisp:970
 437. `ichiran/dict:filter-is-noun`  — fn, dict-grammar.lisp:760
 438. `ichiran/dict:synergy-noun-da`  — fn, dict-grammar.lisp:859
 439. `ichiran/dict:synergy-noun-particle`  — fn, dict-grammar.lisp:850
 440. `ichiran/dict:synergy-o-prefix`  — fn, dict-grammar.lisp:935
 441. `ichiran/dict:synergy-oki`  — fn, dict-grammar.lisp:973
 442. `ichiran/dict:synergy-shicha-ikenai`  — fn, dict-grammar.lisp:951
 443. `ichiran/dict:synergy-shika-negative`  — fn, dict-grammar.lisp:959
 444. `ichiran/dict:synergy-sou-nanda`  — fn, dict-grammar.lisp:878
 445. `ichiran/dict:synergy-suffix-buri`  — fn, dict-grammar.lisp:925
 446. `ichiran/dict:synergy-suffix-chu`  — fn, dict-grammar.lisp:914
 447. `ichiran/dict:synergy-suffix-sei`  — fn, dict-grammar.lisp:929
 448. `ichiran/dict:synergy-suffix-tachi`  — fn, dict-grammar.lisp:921
 449. `ichiran/dict:synergy-to-adverbs`  — fn, dict-grammar.lisp:902
 450. `ichiran/dict:*synergy-list*`  — global, dict-grammar.lisp:723
 451. `ichiran/dict:get-synergies`  — fn, dict-grammar.lisp:976
 452. `ichiran/dict:get-seg-splits`  — fn, dict.lisp:1175
 453. `ichiran/dict:get-segment-score`  — gf, dict.lisp:0
 454. `ichiran/dict:register-item`  — gf, dict.lisp:0
 455. `ichiran/dict:find-best-path`  — fn, dict.lisp:1190
 456. `ichiran/dict:*identical-word-score-cutoff*`  — global, dict.lisp:1020
 457. `ichiran/dict:cull-segments`  — fn, dict.lisp:1027
 458. `ichiran/dict:gen-score`  — fn, dict.lisp:985
 459. `ichiran/dict:*force-kanji-break*`  — global, dict-errata.lisp:1226
 460. `ichiran/dict:*max-word-length*`  — global, dict.lisp:486
 461. `ichiran/dict:*no-kanji-break*`  — global, dict-errata.lisp:1229
 462. `ichiran/dict:*substring-hash*`  — global, dict.lisp:487
 463. `ichiran/dict:*suffix-map-temp*`  — global, dict.lisp:1049
 464. `ichiran/dict:*suffix-next-end*`  — global, dict.lisp:1050
 465. `ichiran/dict:find-sticky-positions`  — fn, dict.lisp:990
 466. `ichiran/dict:find-substring-words`  — fn, dict.lisp:501
 467. `ichiran/dict:verify`  — gf, dict-counters.lisp:0  *[ported]*  *[extracted: counter_2026_05_08]*
 468. `ichiran/numbers:reason`  — gf, numbers.lisp:0
 469. `ichiran/numbers:text`  — gf, numbers.lisp:0
 470. `ichiran/numbers:not-a-number`  — condition, numbers.lisp:0  *[ported]*
 471. `ichiran/dict:find-counter`  — fn, dict-counters.lisp:273  *[ported]*  *[extracted: counter_2026_05_08]*
 472. `ichiran/dict:find-word`  — fn, dict.lisp:489  *[extracted: tatoeba]*
 473. `ichiran/dict:find-word-as-hiragana`  — fn, dict.lisp:592  *[extracted: tatoeba]*
 474. `ichiran/dict:adjoin-word`  — gf, dict.lisp:0
 475. `ichiran/dict:apply-patch`  — fn, dict-grammar.lisp:444
 476. `ichiran/dict:or-as-hiragana`  — fn, dict-grammar.lisp:95
 477. `ichiran/dict:suffix-ra`  — fn, dict-grammar.lisp:516
 478. `ichiran/dict:lex-compare`  — fn, dict-load.lisp:365
 479. `ichiran/dict:pair-words-by-conj`  — fn, dict-grammar.lisp:56
 480. `ichiran/dict:find-word-with-pos`  — fn, dict-grammar.lisp:87
 481. `ichiran/dict:suffix-suru`  — fn, dict-grammar.lisp:441
 482. `ichiran/dict:*suffix-unique-only*`  — global, dict-grammar.lisp:330
 483. `ichiran/dict:match-unique`  — fn, dict-grammar.lisp:702
 484. **CYCLE (49 symbols — port together)**
        - `ichiran/dict:*suffix-list*`  — global, dict-grammar.lisp:329
        - `ichiran/dict:abbr-beba`  — fn, dict-grammar.lisp:658
        - `ichiran/dict:abbr-dewanai`  — fn, dict-grammar.lisp:635
        - `ichiran/dict:abbr-geba`  — fn, dict-grammar.lisp:652
        - `ichiran/dict:abbr-ii`  — fn, dict-grammar.lisp:677
        - `ichiran/dict:abbr-keba`  — fn, dict-grammar.lisp:650
        - `ichiran/dict:abbr-meba`  — fn, dict-grammar.lisp:661
        - `ichiran/dict:abbr-n`  — fn, dict-grammar.lisp:616
        - `ichiran/dict:abbr-nakereba`  — fn, dict-grammar.lisp:627
        - `ichiran/dict:abbr-neba`  — fn, dict-grammar.lisp:655
        - `ichiran/dict:abbr-nee`  — fn, dict-grammar.lisp:596
        - `ichiran/dict:abbr-nx`  — fn, dict-grammar.lisp:605
        - `ichiran/dict:abbr-reba`  — fn, dict-grammar.lisp:647
        - `ichiran/dict:abbr-seba`  — fn, dict-grammar.lisp:666
        - `ichiran/dict:abbr-shimasho`  — fn, dict-grammar.lisp:632
        - `ichiran/dict:abbr-teba`  — fn, dict-grammar.lisp:639
        - `ichiran/dict:find-word-full`  — fn, dict.lisp:1052  *[extracted: tatoeba]*
        - `ichiran/dict:find-word-suffix`  — fn, dict-grammar.lisp:706  *[extracted: tatoeba]*
        - `ichiran/dict:find-word-with-conj-prop`  — fn, dict-grammar.lisp:42
        - `ichiran/dict:find-word-with-conj-type`  — fn, dict-grammar.lisp:51
        - `ichiran/dict:find-word-with-suffix`  — fn, dict-grammar.lisp:100
        - `ichiran/dict:suffix-adv`  — fn, dict-grammar.lisp:472
        - `ichiran/dict:suffix-chau`  — fn, dict-grammar.lisp:427
        - `ichiran/dict:suffix-desho`  — fn, dict-grammar.lisp:541
        - `ichiran/dict:suffix-desu`  — fn, dict-grammar.lisp:525
        - `ichiran/dict:suffix-garu`  — fn, dict-grammar.lisp:504
        - `ichiran/dict:suffix-iadj`  — fn, dict-grammar.lisp:500
        - `ichiran/dict:suffix-kudasai`  — fn, dict-grammar.lisp:412
        - `ichiran/dict:suffix-kurai`  — fn, dict-grammar.lisp:552
        - `ichiran/dict:suffix-neg`  — fn, dict-grammar.lisp:392
        - `ichiran/dict:suffix-rashii`  — fn, dict-grammar.lisp:520
        - `ichiran/dict:suffix-ren`  — fn, dict-grammar.lisp:384
        - `ichiran/dict:suffix-ren-`  — fn, dict-grammar.lisp:387
        - `ichiran/dict:suffix-rou`  — fn, dict-grammar.lisp:470
        - `ichiran/dict:suffix-sa`  — fn, dict-grammar.lisp:490
        - `ichiran/dict:suffix-sou`  — fn, dict-grammar.lisp:454
        - `ichiran/dict:suffix-sou+`  — fn, dict-grammar.lisp:468
        - `ichiran/dict:suffix-sugiru`  — fn, dict-grammar.lisp:475
        - `ichiran/dict:suffix-tai`  — fn, dict-grammar.lisp:379
        - `ichiran/dict:suffix-te`  — fn, dict-grammar.lisp:401
        - `ichiran/dict:suffix-te+space`  — fn, dict-grammar.lisp:410
        - `ichiran/dict:suffix-te-ren`  — fn, dict-grammar.lisp:414
        - `ichiran/dict:suffix-teii`  — fn, dict-grammar.lisp:423
        - `ichiran/dict:suffix-teiru`  — fn, dict-grammar.lisp:405
        - `ichiran/dict:suffix-teiru+`  — fn, dict-grammar.lisp:408
        - `ichiran/dict:suffix-to`  — fn, dict-grammar.lisp:436
        - `ichiran/dict:suffix-tosuru`  — fn, dict-grammar.lisp:549
        - `ichiran/dict:te-check`  — fn, dict-grammar.lisp:395
        - `ichiran/dict:teiru-check`  — fn, dict-grammar.lisp:404
 485. `ichiran/dict:get-suffix-map`  — fn, dict-grammar.lisp:685
 486. `ichiran/dict:join-substring-words*`  — fn, dict.lisp:1069
 487. `ichiran/dict:join-substring-words`  — fn, dict.lisp:1113
 488. `ichiran/dict:dict-segment`  — fn, dict.lisp:1451
 489. `ichiran/dict:simple-segment`  — fn, dict.lisp:1456
 490. `ichiran/dict:get-senses-raw`  — fn, dict.lisp:1458
 491. `ichiran/dict:get-senses`  — fn, dict.lisp:1487
 492. `ichiran/dict:get-senses-str`  — fn, dict.lisp:1495
 493. `ichiran/dict:*suffix-description*`  — global, dict-grammar.lisp:0
 494. `ichiran/dict:get-suffix-description`  — fn, dict-grammar.lisp:160
 495. `ichiran/dict:errata-conj-description-hook`  — fn, dict-errata.lisp:1320
 496. `ichiran/dict:load-conj-description`  — fn, dict-load.lisp:255
 497. `ichiran/dict:get-conj-description`  — fn, dict-load.lisp:255
 498. `ichiran/dict:conj-info-short`  — fn, dict.lisp:275
 499. `ichiran/dict:reading-str*`  — fn, dict.lisp:1580
 500. `ichiran/dict:reading-str-seq`  — fn, dict.lisp:1584
 501. `ichiran/dict:short-sense-str`  — fn, dict.lisp:1562
 502. `ichiran/dict:entry-info-short`  — fn, dict.lisp:1595
 503. `ichiran/dict:conj-type-order`  — fn, dict.lisp:1612
 504. `ichiran/dict:is-rareru`  — fn, dict.lisp:1619
 505. `ichiran/dict:filter-props`  — fn, dict.lisp:1627
 506. `ichiran/dict:select-conjs`  — fn, dict.lisp:1604
 507. `ichiran/dict:select-conjs-and-props`  — fn, dict.lisp:1640
 508. `ichiran/dict:print-conj-info`  — fn, dict.lisp:1649
 509. `ichiran/dict:query-parents-kana`  — fn, dict.lisp:415
 510. `ichiran/dict:best-kanji-conj`  — fn, dict.lisp:457
 511. `ichiran/dict:get-kanji`  — gf, dict.lisp:0
 512. `ichiran/dict:word-info-reading-str`  — fn, dict.lisp:1734
 513. `ichiran/dict:reading-str`  — gf, dict.lisp:0
 514. `ichiran/dict:word-info-str`  — fn, dict.lisp:1747
 515. `ichiran:*hepburn-kana-table*`  — global, romanize.lisp:0
 516. `ichiran:generic-romanization`  — class, romanize.lisp:62
 517. `ichiran:generic-hepburn`  — class, romanize.lisp:103
 518. `ichiran:simplified-hepburn`  — class, romanize.lisp:136
 519. `ichiran:traditional-hepburn`  — class, romanize.lisp:152
 520. `ichiran:*hepburn-traditional*`  — global, romanize.lisp:160
 521. `ichiran:*default-romanization-method*`  — global, romanize.lisp:203
 522. `ichiran:join-parts`  — fn, romanize.lisp:235
 523. `ichiran/dict:simplify-reading-list`  — fn, dict.lisp:1704
 524. `ichiran/dict:map-word-info-kana`  — fn, dict.lisp:1728
 525. `ichiran/dict:*hint-char-map*`  — global, dict-split.lisp:816
 526. `ichiran/dict:strip-hints`  — fn, dict-split.lisp:874
 527. `ichiran/dict:*kana-hint-mod*`  — global, dict-split.lisp:813
 528. `ichiran/dict:*hint-simplify-map*`  — global, dict-split.lisp:818
 529. `ichiran/dict:process-hints`  — fn, dict-split.lisp:872
 530. `ichiran:get-character-classes`  — fn, romanize.lisp:3
 531. `ichiran:r-special`  — gf, romanize.lisp:0
 532. `ichiran:process-iteration-characters`  — fn, romanize.lisp:7  *[ported]*
 533. `ichiran:process-modifiers`  — fn, romanize.lisp:15
 534. `ichiran:*kunrei-siki-kana-table*`  — global, romanize.lisp:0
 535. `ichiran:kunrei-siki`  — class, romanize.lisp:194
 536. `ichiran:r-simplify`  — gf, romanize.lisp:0
 537. `ichiran:leftmost-atom`  — fn, romanize.lisp:25
 538. `ichiran:r-base`  — gf, romanize.lisp:0
 539. **CYCLE (2 symbols — port together)**
        - `ichiran:r-apply`  — gf, romanize.lisp:0
        - `ichiran:romanize-core`  — fn, romanize.lisp:29
 540. `ichiran:romanize-list`  — fn, romanize.lisp:205
 541. `ichiran:romanize-word`  — fn, romanize.lisp:217
 542. `ichiran:romanize-word-info`  — fn, romanize.lisp:248
 543. `ichiran:romanize`  — fn, romanize.lisp:257
 544. `ichiran:romanize*`  — fn, romanize.lisp:273
 545. `ichiran/cli:main`  — fn, cli.lisp:48
 546. `ichiran/conn:*debug*`  — global, conn.lisp:20  *[skip — Debug-flag global gating dp. Replaced by the tracing crate's filter level.]*
 547. `ichiran/conn:def-conn-var`  — macro, conn.lisp:41  *[skip — Macro registering a global into the per-connection variable rebinding list. The cross-DB rebinding pattern is gone — each Ctx owns its caches directly.]*
 548. `ichiran/conn:defcache`  — macro, conn.lisp:135  *[skip — Macro registering a cache + defining init-cache method. Rust shape has no registry; each cache is a typed Ctx field with hand-written accessor.]*
 549. `ichiran/conn:dp`  — fn, conn.lisp:149  *[skip — Debug-printer wrapper around *debug*. Replaced by the tracing crate's emit + filter level.]*
 550. `ichiran/conn:let-db`  — macro, conn.lisp:32  *[skip — Rebinds *connection* for a dynamic scope. Multi-DB usage in Rust is Ctx::from_url(other); no scope-binding macro.]*
 551. `ichiran/conn:load-settings`  — fn, conn.lisp:76  *[skip — Loads settings.lisp and overrides connection from env. No counterpart in Rust — config comes from env (or layered config-crate sources) via Ctx::from_env.]*
 552. `ichiran/conn:with-db`  — macro, conn.lisp:46  *[skip — Rebinds *connection* and re-derives per-conn-var cache for a dynamic scope. Replaced by per-Ctx ownership of pool and caches; multi-DB = construct another Ctx.]*
 553. `ichiran/conn:with-log`  — macro, conn.lisp:86  *[skip — Wraps cl-postgres:*query-log* to a stream for the body. Replaced by sqlx + tracing query logging.]*
 554. `ichiran/custom:*municipality-types*`  — global, dict-custom.lisp:97
 555. `ichiran/custom:*municipality-types-description*`  — global, dict-custom.lisp:107
 556. `ichiran/custom:*municipality-types-order*`  — global, dict-custom.lisp:118
 557. `ichiran/custom:*silent-p*`  — global, dict-custom.lisp:5
 558. `ichiran/custom:as-xml-simple`  — fn, dict-custom.lisp:225
 559. `ichiran/custom:municipality`  — struct, dict-custom.lisp:140  *[ported]*
 560. `ichiran/custom:ward`  — struct, dict-custom.lisp:269  *[ported]*
 561. `ichiran/custom:as-xml`  — gf, dict-custom.lisp:0
 562. `ichiran/custom:custom-source`  — class, dict-custom.lisp:54
 563. `ichiran/custom:csv-loader`  — class, dict-custom.lisp:82
 564. `ichiran/custom:municipality-csv`  — class, dict-custom.lisp:93
 565. `ichiran/custom:source-path`  — fn, dict-custom.lisp:318
 566. `ichiran/custom:ward-csv`  — class, dict-custom.lisp:266
 567. `ichiran/custom:xml-loader`  — class, dict-custom.lisp:59
 568. `ichiran/custom:get-custom-data`  — fn, dict-custom.lisp:322
 569. `ichiran/custom:get-words`  — gf, dict-custom.lisp:0
 570. `ichiran/dict:*pos-with-conj-rules*`  — global, dict-load.lisp:307
 571. `ichiran/dict:*do-not-conjugate*`  — global, dict-load.lisp:303
 572. `ichiran/dict:conjugate-p`  — gf, dict.lisp:0
 573. `ichiran/dict:conjugation-rule`  — struct, dict-load.lisp:262  *[ported]*
 574. `ichiran/dict:construct-conjugation`  — fn, dict-load.lisp:281
 575. `ichiran/dict:load-pos-by-index`  — fn, dict-load.lisp:251
 576. `ichiran/dict:get-pos`  — fn, dict-load.lisp:251
 577. `ichiran/dict:load-pos-index`  — fn, dict-load.lisp:247
 578. `ichiran/dict:get-pos-index`  — fn, dict-load.lisp:247
 579. `ichiran/dict:errata-conj-rules-hook`  — fn, dict-errata.lisp:1329
 580. `ichiran/dict:load-conj-rules`  — fn, dict-load.lisp:265
 581. `ichiran/dict:get-conj-rules`  — fn, dict-load.lisp:265
 582. `ichiran/dict:conjugate-entry-inner`  — fn, dict-load.lisp:314
 583. `ichiran/dict:get-all-readings`  — fn, dict-errata.lisp:257
 584. `ichiran/dict:*secondary-conjugation-types-from*`  — global, dict-load.lisp:312
 585. `ichiran/dict:insert-conjugation`  — fn, dict-load.lisp:375
 586. `ichiran/dict:next-seq`  — fn, dict-load.lisp:110
 587. `ichiran/dict:conjugate-entry-outer`  — fn, dict-load.lisp:342
 588. `ichiran/dict:do-node-list-ord`  — macro, dict-load.lisp:26
 589. `ichiran/dict:node-text`  — fn, dict-load.lisp:14
 590. `ichiran/dict:insert-readings`  — fn, dict-load.lisp:32
 591. `ichiran/dict:insert-sense-traits`  — fn, dict-load.lisp:66
 592. `ichiran/dict:insert-senses`  — fn, dict-load.lisp:71
 593. `ichiran/dict:*secondary-conjugation-types*`  — global, dict-load.lisp:314
 594. `ichiran/dict:load-secondary-conjugations`  — fn, dict-load.lisp:457
 595. `ichiran/dict:load-entry`  — fn, dict-load.lisp:113
 596. `ichiran/custom:insert-entry`  — gf, dict-custom.lisp:0
 597. `ichiran/custom:normalize-geo`  — fn, dict-custom.lisp:176
 598. `ichiran/dict:get-candidates`  — fn, dict.lisp:1904
 599. `ichiran/dict:get-glosses`  — fn, dict.lisp:1892
 600. `ichiran/dict:match-glosses`  — fn, dict.lisp:1921
 601. `ichiran/custom:test-entry`  — gf, dict-custom.lisp:0
 602. `ichiran/dict:sense-exists-p`  — fn, dict-load.lisp:80
 603. `ichiran/dict:add-new-sense`  — fn, dict-load.lisp:91
 604. `ichiran/custom:update-entry`  — gf, dict-custom.lisp:0
 605. `ichiran/custom:update-entry-gloss`  — gf, dict-custom.lisp:0
 606. `ichiran/custom:xml-entry`  — struct, dict-custom.lisp:63  *[skip — XML reader out of scope per project decision (HANDOFF Resolved 2026-05-03); content slot holds a DOM document that cannot be constructed without an XML reader]*
 607. `ichiran/custom:insert`  — gf, dict-custom.lisp:0
 608. `ichiran/custom:municipality-short`  — fn, dict-custom.lisp:123
 609. `ichiran:*hepburn-simple*`  — global, romanize.lisp:146
 610. `ichiran:romanize-word-geo`  — fn, romanize.lisp:232
 611. `ichiran/custom:romanize-municipality`  — fn, dict-custom.lisp:133
 612. `ichiran/custom:process-entry`  — gf, dict-custom.lisp:0
 613. `ichiran/custom:source-file`  — gf, dict-custom.lisp:0
 614. `ichiran/custom:slurp`  — gf, dict-custom.lisp:0
 615. `ichiran/custom:load-custom-data`  — fn, dict-custom.lisp:329
 616. `ichiran/dict:*aux-verbs*`  — global, dict-grammar.lisp:1072
 617. `ichiran/dict:*conj-description*`  — global, dict-load.lisp:0
 618. `ichiran/dict:*conj-rules*`  — global, dict-load.lisp:0
 619. `ichiran/dict:*do-not-conjugate-seq*`  — global, dict-load.lisp:305
 620. `ichiran/dict:*easy-hints-seqs*`  — global, dict-split.lisp:904
 621. `ichiran/dict:*hints-checked*`  — global, dict-split.lisp:947
 622. `ichiran/dict:*honorifics*`  — global, dict-grammar.lisp:1156
 623. `ichiran/dict:*jmdict-data*`  — global, settings.lisp:12
 624. `ichiran/dict:*jmdict-path*`  — global, settings.lisp:10
 625. `ichiran/dict:*kana-hint-map*`  — global, dict-split.lisp:832
 626. `ichiran/dict:*noun-particles*`  — global, dict-grammar.lisp:801
 627. `ichiran/dict:*pos-by-index*`  — global, dict-load.lisp:0
 628. `ichiran/dict:*pos-index*`  — global, dict-load.lisp:0
 629. `ichiran/dict:find-conj`  — fn, dict-errata.lisp:1
 630. `ichiran/dict:add-conj`  — fn, dict-errata.lisp:15
 631. `ichiran/dict:root-diff`  — fn, dict-errata.lisp:95
 632. `ichiran/dict:root-diff-fn`  — fn, dict-errata.lisp:104
 633. `ichiran/dict:add-conj-reading`  — fn, dict-errata.lisp:109
 634. `ichiran/dict:add-reading`  — fn, dict-errata.lisp:35
 635. `ichiran/dict:add-deha-ja-readings`  — fn, dict-errata.lisp:171
 636. `ichiran/dict:add-sense-prop`  — fn, dict-errata.lisp:140
 637. `ichiran/dict:set-reading`  — gf, dict-load.lisp:0
 638. `ichiran/dict:reset-readings`  — fn, dict-errata.lisp:70
 639. `ichiran/dict:delete-reading`  — fn, dict-errata.lisp:76
 640. `ichiran/dict:set-common`  — fn, dict-errata.lisp:166
 641. `ichiran/dict:set-primary-nokanji`  — fn, dict-errata.lisp:224
 642. `ichiran/dict:add-errata-apr19`  — fn, dict-errata.lisp:847
 643. `ichiran/dict:add-new-sense*`  — fn, dict-errata.lisp:153
 644. `ichiran/dict:add-errata-apr20`  — fn, dict-errata.lisp:932
 645. `ichiran/dict:do-readings`  — macro, dict-errata.lisp:246
 646. `ichiran/dict:add-primary-nokanji`  — fn, dict-errata.lisp:251
 647. `ichiran/dict:delete-sense-prop`  — fn, dict-errata.lisp:136
 648. `ichiran/dict:add-errata-aug18`  — fn, dict-errata.lisp:803
 649. `ichiran/dict:add-gloss`  — fn, dict-errata.lisp:156
 650. `ichiran/dict:add-errata-counters`  — fn, dict-errata.lisp:1159
 651. `ichiran/dict:add-errata-dec23`  — fn, dict-errata.lisp:1028
 652. `ichiran/dict:add-errata-feb17`  — fn, dict-errata.lisp:608
 653. `ichiran/dict:add-errata-jan18`  — fn, dict-errata.lisp:697
 654. `ichiran/dict:add-errata-jan19`  — fn, dict-errata.lisp:823
 655. `ichiran/dict:add-errata-jan20`  — fn, dict-errata.lisp:867
 656. `ichiran/dict:replace-reading`  — fn, dict-errata.lisp:49
 657. `ichiran/dict:add-errata-jan21`  — fn, dict-errata.lisp:979
 658. `ichiran/dict:add-errata-jan22`  — fn, dict-errata.lisp:1017
 659. `ichiran/dict:replace-reading-conj`  — fn, dict-errata.lisp:60
 660. `ichiran/dict:add-errata-jan25`  — fn, dict-errata.lisp:1055
 661. `ichiran/dict:add-errata-jan26`  — fn, dict-errata.lisp:1077
 662. `ichiran/dict:rearrange-readings`  — fn, dict-errata.lisp:229
 663. `ichiran/dict:rearrange-readings-conj`  — fn, dict-errata.lisp:241
 664. `ichiran/dict:add-errata-jul20`  — fn, dict-errata.lisp:961
 665. `ichiran/dict:add-errata-mar18`  — fn, dict-errata.lisp:764
 666. `ichiran/dict:add-errata-may21`  — fn, dict-errata.lisp:1006
 667. `ichiran/dict:delete-conjugation`  — fn, dict-errata.lisp:198
 668. `ichiran/dict:add-gozaimasu-conjs`  — fn, dict-errata.lisp:263
 669. `ichiran/dict:conjugate-da`  — fn, dict-errata.lisp:281
 670. `ichiran/dict:delete-senses`  — fn, dict-errata.lisp:129
 671. `ichiran/dict:remove-hiragana-nokanji`  — fn, dict-errata.lisp:217
 672. `ichiran/dict:add-errata`  — fn, dict-errata.lisp:289
 673. `ichiran/dict:add-sense`  — fn, dict-errata.lisp:146
 674. `ichiran/dict:true-kana`  — gf, dict.lisp:0
 675. `ichiran/dict:true-kanji`  — gf, dict.lisp:0
 676. `ichiran/kanji:reading`  — dao, kanji.lisp:42  *[ported]*
 677. `ichiran/kanji:get-reading-alternatives`  — fn, kanji.lisp:216
 678. `ichiran/kanji:*reading-cache*`  — global, kanji.lisp:199
 679. `ichiran/kanji:kanji`  — dao, kanji.lisp:10  *[ported]*
 680. `ichiran/kanji:get-readings-cache`  — fn, kanji.lisp:199
 681. `ichiran/kanji:get-normal-readings`  — fn, kanji.lisp:231
 682. `ichiran/kanji:make-rmap`  — fn, kanji.lisp:273
 683. `ichiran/kanji:match-readings*`  — fn, kanji.lisp:241
 684. `ichiran/kanji:match-readings`  — fn, kanji.lisp:292
 685. `ichiran/dict:check-easy-hints`  — fn, dict-split.lisp:950
 686. `ichiran/dict:common-tags`  — gf, dict.lisp:0
 687. `ichiran/dict:conj-prop-json`  — fn, dict.lisp:283
 688. `ichiran/dict:find-words-seqs`  — fn, dict.lisp:520
 689. `ichiran/dict:get-original-text-once`  — fn, dict.lisp:369
 690. `ichiran/dict:match-kana-kanji`  — fn, dict.lisp:1507
 691. `ichiran/dict:match-sense-restrictions`  — fn, dict.lisp:1515
 692. `ichiran/dict:split-pos`  — fn, dict.lisp:1535
 693. `ichiran/dict:get-senses-json`  — fn, dict.lisp:1537
 694. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:conj-info-json`  — fn, dict.lisp:1698
        - `ichiran/dict:conj-info-json*`  — fn, dict.lisp:1665
 695. `ichiran/dict:conjugate-word`  — fn, dict-load.lisp:294
 696. `ichiran/dict:csv-hash`  — macro, dict-load.lisp:201
 697. `ichiran/dict:defsuffix`  — macro, dict-grammar.lisp:342
 698. `ichiran/dict:def-abbr-suffix`  — macro, dict-grammar.lisp:557
 699. `ichiran/dict:defsplit`  — macro, dict-split.lisp:5
 700. `ichiran/dict:def-simple-split`  — macro, dict-split.lisp:11
 701. `ichiran/dict:def-de-split`  — macro, dict-split.lisp:81
 702. `ichiran/dict:def-do-split`  — macro, dict-split.lisp:181
 703. `ichiran/dict:defhint`  — macro, dict-split.lisp:892
 704. `ichiran/dict:insert-hints`  — fn, dict-split.lisp:875
 705. `ichiran/dict:translate-hint-position`  — fn, dict-split.lisp:930
 706. `ichiran/dict:translate-hints`  — fn, dict-split.lisp:942
 707. `ichiran/dict:def-easy-hint`  — macro, dict-split.lisp:955
 708. `ichiran/dict:defpenalty`  — macro, dict-grammar.lisp:981
 709. `ichiran/dict:def-generic-penalty`  — macro, dict-grammar.lisp:984
 710. `ichiran/dict:defsynergy`  — macro, dict-grammar.lisp:738
 711. `ichiran/dict:def-generic-synergy`  — macro, dict-grammar.lisp:739
 712. `ichiran/dict:def-reader-for-json`  — macro, dict.lisp:1289
 713. `ichiran/dict:defsegfilter`  — macro, dict-grammar.lisp:1043
 714. `ichiran/dict:def-segfilter-must-follow`  — macro, dict-grammar.lisp:1049
 715. `ichiran/dict:def-shi-split`  — macro, dict-split.lisp:191
 716. `ichiran/dict:def-simple-hint`  — macro, dict-split.lisp:901
 717. `ichiran/dict:def-simple-suffix`  — macro, dict-grammar.lisp:345
 718. `ichiran/dict:def-special-counter`  — macro, dict-counters.lisp:361
 719. `ichiran/dict:def-toori-split`  — macro, dict-split.lisp:143
 720. `ichiran/dict:delete-duplicate-props`  — fn, dict.lisp:295
 721. `ichiran/dict:drop-extras`  — fn, dict-load.lisp:194
 722. `ichiran/dict:entry-digest`  — fn, dict.lisp:64
 723. `ichiran/dict:entry-info-long`  — fn, dict.lisp:1601
 724. `ichiran/dict:exists-reading`  — fn, dict.lisp:1847
 725. `ichiran/dict:filter-is-pos`  — macro, dict-grammar.lisp:772
 726. `ichiran/dict:find-word-kana-pattern`  — fn, dict.lisp:1877
 727. `ichiran/dict:find-kanji-for-pattern`  — fn, dict.lisp:1882
 728. `ichiran/dict:find-word-info`  — fn, dict.lisp:1850
 729. `ichiran/dict:word-info-reading`  — fn, dict.lisp:1445
 730. `ichiran/dict:word-info-gloss-json`  — fn, dict.lisp:1784
 731. `ichiran/dict:find-word-info-json`  — fn, dict.lisp:1872
 732. `ichiran/dict:fix-entities`  — fn, dict-load.lisp:159
 733. `ichiran/dict:get-kanji-words`  — fn, dict.lisp:1836
 734. `ichiran/dict:init-tables`  — fn, dict-load.lisp:3
 735. `ichiran/dict:length-multiplier`  — fn, dict.lisp:681
 736. `ichiran/dict:load-best-readings`  — fn, dict-load.lisp:530
 737. `ichiran/dict:load-conjugations`  — fn, dict-load.lisp:445
 738. `ichiran/dict:recalc-entry-stats-all`  — fn, dict.lisp:59
 739. `ichiran/dict:load-extras`  — fn, dict-load.lisp:183
 740. `ichiran/dict:load-jmdict`  — fn, dict-load.lisp:168
 741. `ichiran/dict:recalc-entry-stats`  — fn, dict.lisp:53
 742. `ichiran/dict:word-info-json`  — fn, dict.lisp:1262
 743. `ichiran/dict:simple-word-info`  — fn, dict.lisp:1282
 744. `ichiran/dict:split-kigatsuku`  — fn, dict-split.lisp:298
 745. `ichiran/dict:substring-index`  — fn, dict.lisp:1132
 746. `ichiran/dict:suffix-sou-base`  — macro, dict-grammar.lisp:445
 747. `ichiran/dict:word-info-from-text`  — fn, dict.lisp:1382
 748. `ichiran/dict:word-info-rec-find`  — fn, dict.lisp:1409
 749. `ichiran/dict:word-readings`  — fn, dict.lisp:536
 750. `ichiran/kanji:*kanjidic-path*`  — global, settings.lisp:16
 751. `ichiran/kanji:calculate-perc`  — fn, kanji.lisp:349
 752. `ichiran/kanji:first-node-text`  — fn, kanji.lisp:106
 753. `ichiran/kanji:get-original-reading`  — fn, kanji.lisp:308
 754. `ichiran/kanji:get-reading-stats`  — fn, kanji.lisp:399
 755. `ichiran/kanji:get-readings`  — fn, kanji.lisp:211
 756. `ichiran/kanji:meaning`  — dao, kanji.lisp:83  *[ported]*
 757. `ichiran/kanji:okurigana`  — dao, kanji.lisp:67  *[ported]*
 758. `ichiran/kanji:id`  — gf, kanji.lisp:0
 759. `ichiran/kanji:init-tables`  — fn, kanji.lisp:98
 760. `ichiran/kanji:kanji-id`  — gf, kanji.lisp:0
 761. `ichiran/kanji:stat-common`  — gf, kanji.lisp:0
 762. `ichiran:*hepburn-basic*`  — global, romanize.lisp:144
 763. `ichiran/kanji:reading-info-json`  — fn, kanji.lisp:354
 764. `ichiran/kanji:to-json`  — gf, kanji.lisp:0
 765. `ichiran/kanji:kanji-info-json`  — fn, kanji.lisp:392
 766. `ichiran/kanji:kanji-reading-json`  — fn, kanji.lisp:410
 767. `ichiran/kanji:kanji-word-stats`  — fn, kanji.lisp:316
 768. `ichiran/kanji:load-readings`  — fn, kanji.lisp:114
 769. `ichiran/kanji:load-kanji`  — fn, kanji.lisp:152
 770. `ichiran/kanji:load-kanji-stats`  — fn, kanji.lisp:332
 771. `ichiran/kanji:load-kanjidic`  — fn, kanji.lisp:185
 772. `ichiran/kanji:process-match-json`  — fn, kanji.lisp:428
 773. `ichiran/kanji:match-readings-json`  — fn, kanji.lisp:452
 774. `ichiran/kanji:query-kanji-json`  — macro, kanji.lisp:458
 775. `ichiran/numbers:*digit-kanji-legal*`  — global, numbers.lisp:5  *[ported]*
 776. `ichiran/numbers:parse-number*`  — fn, numbers.lisp:57  *[ported]*
 777. `ichiran/numbers:parse-number`  — fn, numbers.lisp:77  *[ported]*  *[extracted: counter_2026_05_08]*
 778. `ichiran:modified-hepburn`  — class, romanize.lisp:162
 779. `ichiran:*hepburn-modified*`  — global, romanize.lisp:168
 780. `ichiran:*hepburn-passport*`  — global, romanize.lisp:149
 781. `ichiran:*kunrei-siki*`  — global, romanize.lisp:201
 782. `ichiran:rmap-item`  — struct, deromanize.lisp:5  *[ported]*
 783. `ichiran:*romaji-kana*`  — global, deromanize.lisp:0
 784. `ichiran:has-successors`  — fn, deromanize.lisp:11
 785. `ichiran:*romaji-kana-next*`  — global, deromanize.lisp:21
 786. `ichiran:kana-representation`  — struct, deromanize.lisp:23  *[ported]*
 787. `ichiran:possible-long-vowel-p`  — fn, deromanize.lisp:30
 788. `ichiran:apply-rmap-item`  — fn, deromanize.lisp:35
 789. `ichiran:join-branches`  — fn, deromanize.lisp:54
 790. `ichiran:kr-concat`  — fn, deromanize.lisp:23
 791. `ichiran:load-romaji-kana`  — fn, deromanize.lisp:5
 792. `ichiran:get-romaji-kana`  — fn, deromanize.lisp:5
 793. `ichiran:romaji-next`  — fn, deromanize.lisp:46
 794. `ichiran:branches-next`  — fn, deromanize.lisp:69
 795. `ichiran:romaji-kana`  — fn, deromanize.lisp:84
 796. `ichiran:romaji-suggest`  — fn, deromanize.lisp:95
