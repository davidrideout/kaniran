# Port plan — 923 symbols in 862 waves (7 mutual-recursion groups covering 68 symbols)
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
  43. `ichiran/characters:geminate`  — fn, characters.lisp:336  *[ported]*
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
  57. `ichiran/characters:rendaku`  — fn, characters.lisp:320  *[ported]*
  58. `ichiran/characters:safe-subseq`  — fn, characters.lisp:371  *[ported]*
  59. `ichiran/characters:sequential-kanji-positions`  — fn, characters.lisp:207  *[ported]*
  60. `ichiran/characters:unrendaku`  — fn, characters.lisp:308  *[ported]*
  61. `ichiran/characters:voice-char`  — fn, characters.lisp:91  *[ported]*
  62. `ichiran/cli:print-error`  — fn, cli.lisp:37  *[skip — "CLI-only stderr/debugger glue; Rust uses eprintln!/anyhow/panic-hook. Belongs in a future kaniran-cli crate]*
  63. `ichiran/cli:setup-debugger`  — fn, cli.lisp:95  *[skip — "CLI-only stderr/debugger glue; Rust uses eprintln!/anyhow/panic-hook. Belongs in a future kaniran-cli crate]*
  64. `ichiran/conn:cache`  — class, conn.lisp:96  *[skip — "Class with one cached value]*
  65. `ichiran/conn:all-caches`  — fn, conn.lisp:110  *[skip — Class-slot registry pattern doesn't translate. Replaced in Rust by per-cache OnceLock + DI when the DB layer lands; no 1:1 counterpart.]*
  66. `ichiran/conn:get-cache`  — fn, conn.lisp:113  *[skip — Looks up a cache instance from the class-side hash by name. Subsumed by typed-field access on Ctx; no name->instance dispatch.]*
  67. `ichiran/conn:init-cache`  — gf, conn.lisp:0  *[skip — "Generic-function dispatch on a cache name keyword. Per-cache builders become methods on Ctx]*
  68. `ichiran/conn:ensure`  — gf, conn.lisp:0  *[skip — Generic-function lazy-init on a cache name. Per-cache lazy access becomes a method on Ctx over its OnceCell field.]*
  69. `ichiran/conn:reset-cache`  — gf, conn.lisp:0  *[skip — Generic-function force-rebuild of a named cache. Per-cache reset becomes a method on Ctx; no name->instance dispatch.]*
  70. `ichiran/conn:init-all-caches`  — fn, conn.lisp:144  *[skip — Class-slot registry pattern doesn't translate. Replaced in Rust by per-cache OnceLock + DI when the DB layer lands; no 1:1 counterpart.]*
  71. `ichiran/conn:*conn-var-cache*`  — global, conn.lisp:41  *[skip — Cache mapping (var . spec) -> value for the per-connection rebinding. Subsumed by per-Ctx field ownership.]*
  72. `ichiran/conn:*test-var*`  — global, conn.lisp:0  *[skip — Test fixture for the def-conn-var rebinding system; obsolete with per-Ctx ownership.]*
  73. `ichiran/conn:*connection*`  — global, settings.lisp:3  *[skip — Active connection spec global. State lives on Ctx::pool; constructed via Ctx::from_url or Ctx::from_env.]*
  74. `ichiran/dict:*counter-accepts*`  — global, dict-counters.lisp:217  *[ported]*
  75. `ichiran/dict:*counter-foreign*`  — global, dict-counters.lisp:219  *[ported]*
  76. `ichiran/dict:*counter-suffixes*`  — global, dict-counters.lisp:213  *[ported]*
  77. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:counter-text`  — class, dict-counters.lisp:9  *[ported]*
        - `ichiran/dict:number-text`  — class, dict-counters.lisp:203  *[ported]*
  78. `ichiran/dict:counter-age`  — class, dict-counters.lisp:757  *[ported]*
  79. `ichiran/dict:counter-days-kun`  — class, dict-counters.lisp:686  *[ported]*
  80. `ichiran/dict:counter-days-on`  — class, dict-counters.lisp:709  *[ported]*
  81. `ichiran/dict:counter-halfhour`  — class, dict-counters.lisp:391  *[ported]*
  82. `ichiran/dict:counter-hifumi`  — class, dict-counters.lisp:518  *[ported]*
  83. `ichiran/dict:counter-months`  — class, dict-counters.lisp:721  *[ported]*
  84. `ichiran/dict:counter-people`  — class, dict-counters.lisp:735  *[ported]*
  85. `ichiran/dict:counter-tsu`  — class, dict-counters.lisp:497  *[ported]*
  86. `ichiran/dict:counter-wari`  — class, dict-counters.lisp:746  *[ported]*
  87. `ichiran/dict:*special-counters*`  — global, dict-counters.lisp:211  *[ported]*
  88. `ichiran/dict:*extra-counter-ids*`  — global, dict-counters.lisp:310  *[ported]*
  89. `ichiran/dict:*skip-counter-ids*`  — global, dict-counters.lisp:315  *[ported]*
  90. `ichiran/dict:sense-prop`  — dao, dict.lisp:197  *[ported]*
  91. `ichiran/dict:get-counter-ids`  — fn, dict-counters.lisp:285  *[ported]*
  92. `ichiran/dict:get-counter-stags`  — fn, dict-counters.lisp:292  *[ported]*
  93. `ichiran/dict:simple-text`  — class, dict.lisp:69  *[ported]*
  94. `ichiran/dict:kana-text`  — dao, dict.lisp:128  *[ported]*
  95. `ichiran/dict:kanji-text`  — dao, dict.lisp:86  *[ported]*
  96. `ichiran/dict:get-counter-readings`  — fn, dict-counters.lisp:335  *[ported]*
  97. `ichiran/dict:conjugation`  — dao, dict.lisp:238  *[ported]*
  98. `ichiran/dict:sense`  — dao, dict.lisp:166  *[ported]*
  99. `ichiran/dict:entry`  — dao, dict.lisp:26  *[ported]*
 100. `ichiran/dict:no-conj-data`  — fn, dict.lisp:337  *[ported]*
 101. `ichiran/dict:*suffix-cache*`  — global, dict-grammar.lisp:0  *[wip — empty-map stub: populated by wave 127 init-suffixes via init-suffix-hashtables + load-kf + every def-simple-suffix callsite. Replace with proper init when wave 127 lands.]*
 102. `ichiran/dict:*suffix-class*`  — global, dict-grammar.lisp:0  *[wip — empty-map stub: populated by wave 127 init-suffixes via init-suffix-hashtables + load-kf + every def-simple-suffix callsite. Replace with proper init when wave 127 lands.]*
 103. **CYCLE (4 symbols — port together)**
        - `ichiran/conn:*conn-vars*`  — global, conn.lisp:39  *[skip — Registry of per-connection-rebound globals. Unneeded once each Ctx owns its caches directly.]*
        - `ichiran/dict:*counter-cache*`  — global, dict-counters.lisp:0  *[ported]*
        - `ichiran/dict:*is-arch-cache*`  — global, dict.lisp:0  *[ported]*
        - `ichiran/dict:*no-conj-data*`  — global, dict.lisp:0  *[ported]*
 104. `ichiran/conn:*connections*`  — global, settings.lisp:5  *[skip — Alist of secondary connection specs. Replaced by call-site Ctx::from_url(...) per database; no global registry.]*
 105. `ichiran/conn:get-spec`  — fn, conn.lisp:25  *[skip — Lisp dbid-dispatch (nil/list/keyword → connection spec) doesn't translate. Connection registry will be handled via the Rust config crate when the DB layer lands.]*
 106. `ichiran/conn:switch-conn-vars`  — fn, conn.lisp:65  *[skip — Per-connection variable rebinding from *conn-var-cache*. Rust has no dynamic-variable shadowing; replaced by per-Database struct ownership of caches when the DB layer lands. Same family as all-caches / get-spec.]*
 107. `ichiran/dict:init-suffix-hashtables`  — fn, dict-grammar.lisp:6  *[skip — Empty-hashtable initializer for *suffix-cache* / *suffix-class* def-conn-vars. Rust replacement is OnceLock<HashMap> populated on first read; no standalone init verb survives.]*
 108. `ichiran/dict:*init-suffixes-lock*`  — global, dict-grammar.lisp:163  *[skip — SBCL mutex guarding init-suffixes-thread's populator and powering init-suffixes-running-p. Subsumed by OnceLock::get_or_init's built-in once-only synchronization on *suffix-cache* / *suffix-class*; no standalone mutex survives.]*
 109. `ichiran/dict:init-suffixes-running-p`  — fn, dict-grammar.lisp:165  *[skip — Loader-busy predicate over a one-shot init thread + def-conn-var cache. Rust replacement is OnceLock::get().is_some() or eager startup init; the verb has nowhere to live.]*
 110. `ichiran/dict:find-word-seq`  — fn, dict-grammar.lisp:73  *[ported]*
 111. `ichiran/dict:find-word-conj-of`  — fn, dict-grammar.lisp:77  *[ported]*
 112. `ichiran/dict:get-kana-form`  — fn, dict-grammar.lisp:36  *[ported]*
 113. `ichiran/dict:conj-prop`  — dao, dict.lisp:262  *[ported]*
 114. `ichiran/dict:conj-source-reading`  — dao, dict.lisp:309  *[ported]*
 115. `ichiran/dict:conj-data`  — struct, dict.lisp:327  *[ported]*
 116. `ichiran/dict:make-conj-data`  — fn, dict.lisp:325  *[ported]*
 117. `ichiran/dict:get-conj-data`  — fn, dict.lisp:340  *[ported]*
 118. `ichiran/dict:*weak-conj-forms*`  — global, dict-errata.lisp:1316  *[ported]*
 119. `ichiran/dict:conj-data-prop`  — fn, dict.lisp:325  *[ported]*
 120. `ichiran/dict:*skip-conj-forms*`  — global, dict-errata.lisp:1310  *[ported]*
 121. `ichiran/dict:test-conj-prop`  — fn, dict-errata.lisp:1336  *[ported]*
 122. `ichiran/dict:skip-by-conj-data`  — fn, dict-errata.lisp:1336  *[ported]*  *[extracted: tatoeba]*  *[audited 33168/33168]*
 123. `ichiran/dict:get-kana-forms-conj-data-filter`  — fn, dict-grammar.lisp:10  *[ported]*  *[extracted: init-suffixes]*
 124. `ichiran/dict:get-kana-forms*`  — fn, dict-grammar.lisp:17  *[ported]*  *[extracted: init-suffixes]*
 125. `ichiran/dict:get-kana-forms`  — fn, dict-grammar.lisp:32  *[ported]*  *[extracted: init-suffixes]*
 126. `ichiran/dict:init-suffixes-thread`  — fn, dict-grammar.lisp:169  *[ported]*
 127. `ichiran/dict:init-suffixes`  — fn, dict-grammar.lisp:332  *[skip — Subsumed by KaniranContext eager construction; init-suffixes-thread (wave 126) is the actual populator. Same prior-art as init-suffixes-running-p.]*
 128. `ichiran/cli:build`  — fn, cli.lisp:102  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 129. `ichiran/cli:print-romanize-info`  — fn, cli.lisp:44  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 130. `ichiran/cli:unknown-option`  — fn, cli.lisp:33  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 131. `ichiran/conn:*is-dynamic-connection*`  — global, conn.lisp:14  *[skip — "Boolean marking 'connection came from env]*
 132. `ichiran/conn:*connection-env-var*`  — global, conn.lisp:13  *[ported]*
 133. `ichiran/conn:get-ichiran-connection-env`  — fn, conn.lisp:154  *[ported]*
 134. `ichiran/conn:load-connection-from-env`  — fn, conn.lisp:166  *[skip — "Side-effects-on-globals semantics (set *connection*]*
 135. `ichiran/dict:process-word-info`  — fn, dict.lisp:1417  *[ported]*
 136. `ichiran/dict:segment-list`  — struct, dict.lisp:1038
 137. `ichiran/dict:segment-list-end`  — fn, dict.lisp:1038
 138. `ichiran/dict:segment-list-start`  — fn, dict.lisp:1038
 139. `ichiran/dict:*disable-hints*`  — global, dict.lisp:78  *[ported]*
 140. `ichiran/dict:*kana-hint-space*`  — global, dict-split.lisp:814  *[ported]*
 141. `ichiran/dict:query-parents-kanji`  — fn, dict.lisp:400  *[extracted: tatoeba]*
 142. `ichiran/dict:best-kana-conj`  — fn, dict.lisp:428  *[extracted: tatoeba]*
 143. `ichiran/dict:get-digit`  — fn, dict-counters.lisp:94  *[ported]*  *[extracted: tatoeba]*  *[audited 193/193]*
 144. `ichiran/numbers:*digit-to-kana*`  — global, numbers.lisp:25  *[ported]*
 145. `ichiran/numbers:*power-to-kana*`  — global, numbers.lisp:28  *[ported]*
 146. `ichiran/dict:counter-join`  — gf, dict-counters.lisp:0
 147. `ichiran/dict:*hint-map*`  — global, dict-split.lisp:850
 148. `ichiran/dict:conj-data-from`  — fn, dict.lisp:325  *[ported]*
 149. `ichiran/dict:get-kanji-kana-old`  — fn, dict.lisp:115  *[extracted: tatoeba]*
 150. `ichiran/numbers:*char-number-class*`  — global, numbers.lisp:9  *[ported]*
 151. `ichiran/numbers:*char-number-class-hash*`  — global, numbers.lisp:18  *[ported]*
 152. `ichiran/numbers:num-sandhi`  — gf, numbers.lisp:0  *[ported]*
 153. `ichiran/numbers:group-to-kana`  — fn, numbers.lisp:117  *[ported]*
 154. `ichiran/numbers:*digit-kanji-default*`  — global, numbers.lisp:3  *[ported]*
 155. `ichiran/numbers:*power-kanji*`  — global, numbers.lisp:7  *[ported]*
 156. `ichiran/numbers:number-to-kanji`  — fn, numbers.lisp:35  *[ported]*
 157. `ichiran/numbers:number-to-kana`  — fn, numbers.lisp:125  *[ported]*
 158. **CYCLE (7 symbols — port together)**
        - `ichiran/dict:compound-text`  — class, dict.lisp:608  *[ported]*
        - `ichiran/dict:get-hint`  — fn, dict-split.lisp:968
        - `ichiran/dict:get-kana`  — gf, dict.lisp:0
        - `ichiran/dict:proxy-text`  — class, dict.lisp:550  *[ported]*
        - `ichiran/dict:score-base`  — gf, dict.lisp:0
        - `ichiran/dict:true-text`  — gf, dict.lisp:0
        - `ichiran/dict:word-conj-data`  — gf, dict.lisp:0
 159. `ichiran/dict:word-info`  — class, dict.lisp:1245  *[ported]*
 160. `ichiran/dict:*segment-score-cutoff*`  — global, dict.lisp:1351
 161. `ichiran/dict:segment-list-matches`  — fn, dict.lisp:1038
 162. `ichiran/dict:segment-list-segments`  — fn, dict.lisp:1038
 163. `ichiran/dict:segment`  — struct, dict.lisp:674
 164. `ichiran/dict:segment-text`  — fn, dict.lisp:674
 165. `ichiran/dict:segment-word`  — fn, dict.lisp:674
 166. `ichiran/dict:get-text`  — gf, dict.lisp:0
 167. `ichiran/dict:segment-end`  — fn, dict.lisp:674
 168. `ichiran/dict:segment-score`  — fn, dict.lisp:674
 169. `ichiran/dict:segment-start`  — fn, dict.lisp:674
 170. `ichiran/dict:ordinal-str`  — fn, dict-counters.lisp:38
 171. `ichiran/dict:value-string`  — gf, dict-counters.lisp:0
 172. `ichiran/dict:word-type`  — gf, dict.lisp:0
 173. `ichiran/dict:word-info-from-segment`  — fn, dict.lisp:1327
 174. `ichiran/dict:word-info-from-segment-list`  — fn, dict.lisp:1353
 175. `ichiran/dict:fill-segment-path`  — fn, dict.lisp:1390
 176. `ichiran/dict:split-1010105`  — fn, dict-split.lisp:771
 177. `ichiran/dict:split-1567610`  — fn, dict-split.lisp:771
 178. `ichiran/dict:split-1675330`  — fn, dict-split.lisp:771
 179. `ichiran/dict:split-2841254`  — fn, dict-split.lisp:771
 180. `ichiran/dict:split-dakara`  — fn, dict-split.lisp:771
 181. `ichiran/dict:split-deha`  — fn, dict-split.lisp:771
 182. `ichiran/dict:split-dokoroka`  — fn, dict-split.lisp:771
 183. `ichiran/dict:split-hitorashii`  — fn, dict-split.lisp:771
 184. `ichiran/dict:split-honno`  — fn, dict-split.lisp:771
 185. `ichiran/dict:split-kanatte`  — fn, dict-split.lisp:771
 186. `ichiran/dict:split-naito`  — fn, dict-split.lisp:771
 187. `ichiran/dict:split-omise`  — fn, dict-split.lisp:771
 188. `ichiran/dict:split-toha`  — fn, dict-split.lisp:771
 189. `ichiran/dict:split-tokorode`  — fn, dict-split.lisp:771
 190. `ichiran/dict:split-tokorodewa`  — fn, dict-split.lisp:771
 191. `ichiran/dict:split-tokoroe`  — fn, dict-split.lisp:771
 192. `ichiran/dict:split-tokoroga`  — fn, dict-split.lisp:771
 193. `ichiran/dict:split-tokorowo`  — fn, dict-split.lisp:771
 194. `ichiran/dict:*segsplit-map*`  — global, dict-split.lisp:704
 195. `ichiran/dict:split-1000430`  — fn, dict-split.lisp:505
 196. `ichiran/dict:split-1002970`  — fn, dict-split.lisp:492
 197. `ichiran/dict:split-1005600`  — fn, dict-split.lisp:498
 198. `ichiran/dict:split-1006280`  — fn, dict-split.lisp:669
 199. `ichiran/dict:split-1006880`  — fn, dict-split.lisp:727
 200. `ichiran/dict:split-1008030`  — fn, dict-split.lisp:645
 201. `ichiran/dict:split-1207840`  — fn, dict-split.lisp:711
 202. `ichiran/dict:split-1221530`  — fn, dict-split.lisp:611
 203. `ichiran/dict:split-1221680`  — fn, dict-split.lisp:521
 204. `ichiran/dict:split-1314600`  — fn, dict-split.lisp:512
 205. `ichiran/dict:split-1314770`  — fn, dict-split.lisp:640
 206. `ichiran/dict:split-1315860`  — fn, dict-split.lisp:535
 207. `ichiran/dict:split-1322540`  — fn, dict-split.lisp:517
 208. `ichiran/dict:split-1322560`  — fn, dict-split.lisp:719
 209. `ichiran/dict:split-1327220`  — fn, dict-split.lisp:424
 210. `ichiran/dict:split-1327230`  — fn, dict-split.lisp:429
 211. `ichiran/dict:split-1349300`  — fn, dict-split.lisp:608
 212. `ichiran/dict:split-1362970`  — fn, dict-split.lisp:759
 213. `ichiran/dict:split-1474200`  — fn, dict-split.lisp:546
 214. `ichiran/dict:split-1502500`  — fn, dict-split.lisp:487
 215. `ichiran/dict:split-1508380`  — fn, dict-split.lisp:478
 216. `ichiran/dict:split-1532270`  — fn, dict-split.lisp:685
 217. `ichiran/dict:split-1538340`  — fn, dict-split.lisp:526
 218. `ichiran/dict:split-1551500`  — fn, dict-split.lisp:631
 219. `ichiran/dict:split-1579130`  — fn, dict-split.lisp:559
 220. `ichiran/dict:split-1581550`  — fn, dict-split.lisp:650
 221. `ichiran/dict:split-1591050`  — fn, dict-split.lisp:571
 222. `ichiran/dict:split-1591980`  — fn, dict-split.lisp:625
 223. `ichiran/dict:split-1597740`  — fn, dict-split.lisp:645
 224. `ichiran/dict:split-1601010`  — fn, dict-split.lisp:732
 225. `ichiran/dict:split-1601080`  — fn, dict-split.lisp:658
 226. `ichiran/dict:split-1602740`  — fn, dict-split.lisp:605
 227. `ichiran/dict:split-1606530`  — fn, dict-split.lisp:676
 228. `ichiran/dict:split-1606800`  — fn, dict-split.lisp:706
 229. `ichiran/dict:split-1612640`  — fn, dict-split.lisp:509
 230. `ichiran/dict:split-1774820`  — fn, dict-split.lisp:756
 231. `ichiran/dict:split-1854750`  — fn, dict-split.lisp:596
 232. `ichiran/dict:split-1855670`  — fn, dict-split.lisp:742
 233. `ichiran/dict:split-1863230`  — fn, dict-split.lisp:698
 234. `ichiran/dict:split-1881690`  — fn, dict-split.lisp:734
 235. `ichiran/dict:optprefix`  — fn, dict-split.lisp:580
 236. `ichiran/dict:split-1894260`  — fn, dict-split.lisp:586
 237. `ichiran/dict:split-2002270`  — fn, dict-split.lisp:633
 238. `ichiran/dict:split-2007500`  — fn, dict-split.lisp:681
 239. `ichiran/dict:split-2009290`  — fn, dict-split.lisp:483
 240. `ichiran/dict:split-2016840`  — fn, dict-split.lisp:502
 241. `ichiran/dict:split-2026650`  — fn, dict-split.lisp:601
 242. `ichiran/dict:split-2083990`  — fn, dict-split.lisp:468
 243. `ichiran/dict:split-2088480`  — fn, dict-split.lisp:438
 244. `ichiran/dict:split-2109610`  — fn, dict-split.lisp:715
 245. `ichiran/dict:split-2133750`  — fn, dict-split.lisp:691
 246. `ichiran/dict:split-2272780`  — fn, dict-split.lisp:616
 247. `ichiran/dict:split-2276360`  — fn, dict-split.lisp:554
 248. `ichiran/dict:split-2433760`  — fn, dict-split.lisp:432
 249. `ichiran/dict:split-2526850`  — fn, dict-split.lisp:597
 250. `ichiran/dict:split-2529050`  — fn, dict-split.lisp:662
 251. `ichiran/dict:split-2666360`  — fn, dict-split.lisp:446
 252. `ichiran/dict:split-2668400`  — fn, dict-split.lisp:564
 253. `ichiran/dict:split-2724560`  — fn, dict-split.lisp:442
 254. `ichiran/dict:split-2757500`  — fn, dict-split.lisp:531
 255. `ichiran/dict:split-2757540`  — fn, dict-split.lisp:673
 256. `ichiran/dict:split-2762260`  — fn, dict-split.lisp:474
 257. `ichiran/dict:split-2771940`  — fn, dict-split.lisp:457
 258. `ichiran/dict:split-2834051`  — fn, dict-split.lisp:702
 259. `ichiran/dict:split-2834732`  — fn, dict-split.lisp:740
 260. `ichiran/dict:split-2835890`  — fn, dict-split.lisp:577
 261. `ichiran/dict:split-2846470`  — fn, dict-split.lisp:621
 262. `ichiran/dict:split-2855921`  — fn, dict-split.lisp:748
 263. `ichiran/dict:split-de-1004800`  — fn, dict-split.lisp:104
 264. `ichiran/dict:split-de-1006840`  — fn, dict-split.lisp:106
 265. `ichiran/dict:split-de-1163700`  — fn, dict-split.lisp:102
 266. `ichiran/dict:split-de-1189420`  — fn, dict-split.lisp:111
 267. `ichiran/dict:split-de-1245390`  — fn, dict-split.lisp:108
 268. `ichiran/dict:split-de-1270210`  — fn, dict-split.lisp:140
 269. `ichiran/dict:split-de-1272220`  — fn, dict-split.lisp:112
 270. `ichiran/dict:split-de-1311360`  — fn, dict-split.lisp:113
 271. `ichiran/dict:split-de-1343110`  — fn, dict-split.lisp:139
 272. `ichiran/dict:split-de-1368500`  — fn, dict-split.lisp:114
 273. `ichiran/dict:split-de-1395670`  — fn, dict-split.lisp:115
 274. `ichiran/dict:split-de-1417790`  — fn, dict-split.lisp:116
 275. `ichiran/dict:split-de-1454270`  — fn, dict-split.lisp:117
 276. `ichiran/dict:split-de-1479100`  — fn, dict-split.lisp:119
 277. `ichiran/dict:split-de-1510140`  — fn, dict-split.lisp:120
 278. `ichiran/dict:split-de-1518550`  — fn, dict-split.lisp:121
 279. `ichiran/dict:split-de-1530610`  — fn, dict-split.lisp:107
 280. `ichiran/dict:split-de-1531420`  — fn, dict-split.lisp:122
 281. `ichiran/dict:split-de-1597400`  — fn, dict-split.lisp:123
 282. `ichiran/dict:split-de-1611020`  — fn, dict-split.lisp:102
 283. `ichiran/dict:split-de-1679990`  — fn, dict-split.lisp:124
 284. `ichiran/dict:split-de-1682060`  — fn, dict-split.lisp:126
 285. `ichiran/dict:split-de-1736650`  — fn, dict-split.lisp:127
 286. `ichiran/dict:split-de-1865020`  — fn, dict-split.lisp:128
 287. `ichiran/dict:split-de-1878880`  — fn, dict-split.lisp:129
 288. `ichiran/dict:split-de-2126220`  — fn, dict-split.lisp:130
 289. `ichiran/dict:split-de-2136520`  — fn, dict-split.lisp:131
 290. `ichiran/dict:split-de-2513590`  — fn, dict-split.lisp:133
 291. `ichiran/dict:split-de-2719270`  — fn, dict-split.lisp:109
 292. `ichiran/dict:split-de-2771850`  — fn, dict-split.lisp:135
 293. `ichiran/dict:split-de-2810720`  — fn, dict-split.lisp:105
 294. `ichiran/dict:split-de-2810800`  — fn, dict-split.lisp:136
 295. `ichiran/dict:split-degozaimasu`  — fn, dict-split.lisp:140
 296. `ichiran/dict:split-desura`  — fn, dict-split.lisp:382
 297. `ichiran/dict:split-do-2142680`  — fn, dict-split.lisp:190
 298. `ichiran/dict:split-do-2142710`  — fn, dict-split.lisp:189
 299. `ichiran/dict:split-do-2523480`  — fn, dict-split.lisp:190
 300. `ichiran/dict:split-do-2803190`  — fn, dict-split.lisp:189
 301. `ichiran/dict:split-dogatsukeru`  — fn, dict-split.lisp:276
 302. `ichiran/dict:split-gotoni`  — fn, dict-split.lisp:387
 303. `ichiran/dict:split-hairikomeru`  — fn, dict-split.lisp:340
 304. `ichiran/dict:split-hajiketobu`  — fn, dict-split.lisp:328
 305. `ichiran/dict:split-hajikidasu`  — fn, dict-split.lisp:368
 306. `ichiran/dict:split-hayaimonode`  — fn, dict-split.lisp:267
 307. `ichiran/dict:split-hisshininatte`  — fn, dict-split.lisp:348
 308. `ichiran/dict:split-hitotachi`  — fn, dict-split.lisp:375
 309. `ichiran/dict:split-jan`  — fn, dict-split.lisp:454
 310. `ichiran/dict:split-janai`  — fn, dict-split.lisp:449
 311. `ichiran/dict:split-janaika`  — fn, dict-split.lisp:281
 312. `ichiran/dict:split-kaasan`  — fn, dict-split.lisp:285
 313. `ichiran/dict:split-kaisasae`  — fn, dict-split.lisp:399
 314. `ichiran/dict:split-katawonaraberu`  — fn, dict-split.lisp:305
 315. `ichiran/dict:split-kawaribae`  — fn, dict-split.lisp:258
 316. `ichiran/dict:split-kimatte`  — fn, dict-split.lisp:314
 317. `ichiran/dict:split-kinosei`  — fn, dict-split.lisp:295
 318. `ichiran/dict:split-kotonisuru`  — fn, dict-split.lisp:360
 319. `ichiran/dict:split-motteiku`  — fn, dict-split.lisp:333
 320. `ichiran/dict:split-moushiwakenasasou`  — fn, dict-split.lisp:310
 321. `ichiran/dict:split-nakunaru`  — fn, dict-split.lisp:237
 322. `ichiran/dict:split-nakunaru2`  — fn, dict-split.lisp:244
 323. `ichiran/dict:split-nanimokamo`  — fn, dict-split.lisp:301
 324. `ichiran/dict:split-nantokanaru`  — fn, dict-split.lisp:323
 325. `ichiran/dict:split-nara`  — fn, dict-split.lisp:464
 326. `ichiran/dict:split-nitotte`  — fn, dict-split.lisp:354
 327. `ichiran/dict:split-osagari`  — fn, dict-split.lisp:395
 328. `ichiran/dict:split-osoreiru`  — fn, dict-split.lisp:318
 329. `ichiran/dict:split-shi-1005700`  — fn, dict-split.lisp:209
 330. `ichiran/dict:split-shi-1005830`  — fn, dict-split.lisp:210
 331. `ichiran/dict:split-shi-1157200`  — fn, dict-split.lisp:211
 332. `ichiran/dict:split-shi-1157220`  — fn, dict-split.lisp:212
 333. `ichiran/dict:split-shi-1157230`  — fn, dict-split.lisp:213
 334. `ichiran/dict:split-shi-1157240`  — fn, dict-split.lisp:232
 335. `ichiran/dict:split-shi-1157280`  — fn, dict-split.lisp:214
 336. `ichiran/dict:split-shi-1157310`  — fn, dict-split.lisp:215
 337. `ichiran/dict:split-shi-1304820`  — fn, dict-split.lisp:234
 338. `ichiran/dict:split-shi-1304890`  — fn, dict-split.lisp:216
 339. `ichiran/dict:split-shi-1304960`  — fn, dict-split.lisp:218
 340. `ichiran/dict:split-shi-1305110`  — fn, dict-split.lisp:219
 341. `ichiran/dict:split-shi-1305280`  — fn, dict-split.lisp:221
 342. `ichiran/dict:split-shi-1305290`  — fn, dict-split.lisp:223
 343. `ichiran/dict:split-shi-1594300`  — fn, dict-split.lisp:223
 344. `ichiran/dict:split-shi-1594310`  — fn, dict-split.lisp:225
 345. `ichiran/dict:split-shi-1594460`  — fn, dict-split.lisp:227
 346. `ichiran/dict:split-shi-1594580`  — fn, dict-split.lisp:228
 347. `ichiran/dict:split-shi-2518250`  — fn, dict-split.lisp:231
 348. `ichiran/dict:split-shi-2858937`  — fn, dict-split.lisp:235
 349. `ichiran/dict:split-shinikakaru`  — fn, dict-split.lisp:345
 350. `ichiran/dict:split-souda`  — fn, dict-split.lisp:290
 351. `ichiran/dict:split-soudesu`  — fn, dict-split.lisp:292
 352. `ichiran/dict:split-tegakakaru`  — fn, dict-split.lisp:249
 353. `ichiran/dict:split-toiu`  — fn, dict-split.lisp:404
 354. `ichiran/dict:split-toiukotoda`  — fn, dict-split.lisp:407
 355. `ichiran/dict:split-tonaru`  — fn, dict-split.lisp:419
 356. `ichiran/dict:split-tonattara`  — fn, dict-split.lisp:415
 357. `ichiran/dict:split-toori-1164910`  — fn, dict-split.lisp:174
 358. `ichiran/dict:split-toori-1260990`  — fn, dict-split.lisp:155
 359. `ichiran/dict:split-toori-1368820`  — fn, dict-split.lisp:171
 360. `ichiran/dict:split-toori-1414570`  — fn, dict-split.lisp:157
 361. `ichiran/dict:split-toori-1424950`  — fn, dict-split.lisp:159
 362. `ichiran/dict:split-toori-1424960`  — fn, dict-split.lisp:161
 363. `ichiran/dict:split-toori-1462720`  — fn, dict-split.lisp:179
 364. `ichiran/dict:split-toori-1489800`  — fn, dict-split.lisp:167
 365. `ichiran/dict:split-toori-1523010`  — fn, dict-split.lisp:169
 366. `ichiran/dict:split-toori-1550490`  — fn, dict-split.lisp:172
 367. `ichiran/dict:split-toori-1619440`  — fn, dict-split.lisp:173
 368. `ichiran/dict:split-toori-1808080`  — fn, dict-split.lisp:171
 369. `ichiran/dict:split-toori-1820790`  — fn, dict-split.lisp:165
 370. `ichiran/dict:*split-map*`  — global, dict-split.lisp:5
 371. `ichiran/dict:*copulae*`  — global, dict-errata.lisp:1205
 372. `ichiran/dict:*final-prt*`  — global, dict-errata.lisp:1182
 373. `ichiran/dict:*non-final-prt*`  — global, dict-errata.lisp:1209
 374. `ichiran/dict:*semi-final-prt*`  — global, dict-errata.lisp:1196
 375. `ichiran/dict:*skip-words*`  — global, dict-errata.lisp:1155
 376. `ichiran/dict:apply-score-mod`  — gf, dict.lisp:0
 377. `ichiran/dict:compare-common`  — fn, dict.lisp:1022
 378. `ichiran/dict:conj-data-via`  — fn, dict.lisp:325
 379. `ichiran/dict:get-non-arch-posi`  — fn, dict.lisp:762
 380. `ichiran/dict:conj-data-src-map`  — fn, dict.lisp:325
 381. `ichiran/dict:get-original-text*`  — fn, dict.lisp:378
 382. `ichiran/dict:get-original-text`  — gf, dict.lisp:0
 383. `ichiran/dict:get-split*`  — fn, dict-split.lisp:67
 384. `ichiran/dict:get-split`  — fn, dict-split.lisp:75
 385. `ichiran/dict:is-arch`  — fn, dict.lisp:760  *[ported]*
 386. `ichiran/dict:*no-kanji-break-penalty*`  — global, dict-errata.lisp:1214
 387. `ichiran/dict:*score-cutoff*`  — global, dict.lisp:1069
 388. `ichiran/dict:parse-suffix-val`  — fn, dict-grammar.lisp:679
 389. `ichiran/dict:make-slice`  — fn, dict.lisp:1010
 390. `ichiran/dict:subseq-slice`  — fn, dict.lisp:1013
 391. `ichiran/dict:get-suffixes`  — fn, dict-grammar.lisp:697
 392. `ichiran/dict:*length-coeff-sequences*`  — global, dict.lisp:686
 393. `ichiran/dict:length-multiplier-coeff`  — fn, dict.lisp:694
 394. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:calc-score`  — fn, dict.lisp:775
        - `ichiran/dict:kanji-break-penalty`  — fn, dict.lisp:702
 395. `ichiran/dict:copy-segment`  — fn, dict.lisp:674
 396. `ichiran/dict:segment-info`  — fn, dict.lisp:674
 397. `ichiran/dict:get-segsplit`  — fn, dict-split.lisp:823
 398. `ichiran/dict:expand-segment-list`  — fn, dict.lisp:1180
 399. `ichiran/dict:*gap-penalty*`  — global, dict.lisp:1165
 400. `ichiran/dict:gap-penalty`  — fn, dict.lisp:1169
 401. `ichiran/dict:top-array`  — class, dict.lisp:1140
 402. `ichiran/dict:get-array`  — gf, dict.lisp:0
 403. `ichiran/dict:classify`  — fn, dict-grammar.lisp:1046
 404. `ichiran/dict:filter-in-seq-set`  — fn, dict-grammar.lisp:783
 405. `ichiran/dict:filter-is-conjugation`  — fn, dict-grammar.lisp:797
 406. `ichiran/dict:copy-segment-list`  — fn, dict.lisp:1038
 407. `ichiran/dict:make-segment-list-from`  — fn, dict-grammar.lisp:733
 408. `ichiran/dict:segfilter-aux-verb`  — fn, dict-grammar.lisp:1099
 409. `ichiran/dict:filter-is-compound-end-text`  — fn, dict-grammar.lisp:820
 410. `ichiran/dict:segfilter-badend`  — fn, dict-grammar.lisp:1114
 411. `ichiran/dict:segfilter-dashi`  — fn, dict-grammar.lisp:1167
 412. `ichiran/dict:segfilter-dekiru`  — fn, dict-grammar.lisp:1175
 413. `ichiran/dict:segfilter-honorific`  — fn, dict-grammar.lisp:1177
 414. `ichiran/dict:filter-is-compound-end`  — fn, dict-grammar.lisp:806
 415. `ichiran/dict:segfilter-janai`  — fn, dict-grammar.lisp:1146
 416. `ichiran/dict:segfilter-mononi`  — fn, dict-grammar.lisp:1177
 417. `ichiran/dict:filter-in-seq-set-simple`  — fn, dict-grammar.lisp:787
 418. `ichiran/dict:segfilter-n`  — fn, dict-grammar.lisp:1106
 419. `ichiran/dict:segfilter-nohayamete`  — fn, dict-grammar.lisp:1151
 420. `ichiran/dict:segfilter-roku`  — fn, dict-grammar.lisp:1129
 421. `ichiran/dict:segfilter-sae`  — fn, dict-grammar.lisp:1141
 422. `ichiran/dict:segfilter-sukiyoki`  — fn, dict-grammar.lisp:1119
 423. `ichiran/dict:segfilter-toomou`  — fn, dict-grammar.lisp:1156
 424. `ichiran/dict:segfilter-totte`  — fn, dict-grammar.lisp:1165
 425. `ichiran/dict:segfilter-tsu-iru`  — fn, dict-grammar.lisp:1101
 426. `ichiran/dict:segfilter-wokarasu`  — fn, dict-grammar.lisp:1112
 427. `ichiran/dict:*segfilter-list*`  — global, dict-grammar.lisp:1024
 428. `ichiran/dict:apply-segfilters`  — fn, dict-grammar.lisp:1177
 429. `ichiran/dict:get-seg-initial`  — fn, dict.lisp:1172
 430. `ichiran/dict:synergy`  — struct, dict-grammar.lisp:713  *[ported]*
 431. `ichiran/dict:make-synergy`  — fn, dict-grammar.lisp:727
 432. `ichiran/dict:penalty-semi-final`  — fn, dict-grammar.lisp:1022
 433. `ichiran/dict:filter-short-kana`  — fn, dict-grammar.lisp:1008
 434. `ichiran/dict:penalty-short`  — fn, dict-grammar.lisp:1020
 435. `ichiran/dict:*penalty-list*`  — global, dict-grammar.lisp:964
 436. `ichiran/dict:get-penalties`  — fn, dict-grammar.lisp:1030
 437. `ichiran/dict:synergy-kanji-prefix`  — fn, dict-grammar.lisp:940
 438. `ichiran/dict:synergy-na-adjectives`  — fn, dict-grammar.lisp:892
 439. `ichiran/dict:synergy-no-adjectives`  — fn, dict-grammar.lisp:884
 440. `ichiran/dict:synergy-no-da`  — fn, dict-grammar.lisp:871
 441. `ichiran/dict:synergy-no-toori`  — fn, dict-grammar.lisp:970
 442. `ichiran/dict:filter-is-noun`  — fn, dict-grammar.lisp:760
 443. `ichiran/dict:synergy-noun-da`  — fn, dict-grammar.lisp:859
 444. `ichiran/dict:synergy-noun-particle`  — fn, dict-grammar.lisp:850
 445. `ichiran/dict:synergy-o-prefix`  — fn, dict-grammar.lisp:935
 446. `ichiran/dict:synergy-oki`  — fn, dict-grammar.lisp:973
 447. `ichiran/dict:synergy-shicha-ikenai`  — fn, dict-grammar.lisp:951
 448. `ichiran/dict:synergy-shika-negative`  — fn, dict-grammar.lisp:959
 449. `ichiran/dict:synergy-sou-nanda`  — fn, dict-grammar.lisp:878
 450. `ichiran/dict:synergy-suffix-buri`  — fn, dict-grammar.lisp:925
 451. `ichiran/dict:synergy-suffix-chu`  — fn, dict-grammar.lisp:914
 452. `ichiran/dict:synergy-suffix-sei`  — fn, dict-grammar.lisp:929
 453. `ichiran/dict:synergy-suffix-tachi`  — fn, dict-grammar.lisp:921
 454. `ichiran/dict:synergy-to-adverbs`  — fn, dict-grammar.lisp:902
 455. `ichiran/dict:*synergy-list*`  — global, dict-grammar.lisp:723
 456. `ichiran/dict:get-synergies`  — fn, dict-grammar.lisp:976
 457. `ichiran/dict:get-seg-splits`  — fn, dict.lisp:1175
 458. `ichiran/dict:synergy-score`  — fn, dict-grammar.lisp:727
 459. `ichiran/dict:get-segment-score`  — gf, dict.lisp:0
 460. `ichiran/dict:top-array-item`  — struct, dict.lisp:1138
 461. `ichiran/dict:make-top-array-item`  — fn, dict.lisp:1138
 462. `ichiran/dict:tai-score`  — fn, dict.lisp:1138
 463. `ichiran/dict:register-item`  — gf, dict.lisp:0
 464. `ichiran/dict:segment-list-top`  — fn, dict.lisp:1038
 465. `ichiran/dict:tai-payload`  — fn, dict.lisp:1138
 466. `ichiran/dict:find-best-path`  — fn, dict.lisp:1190
 467. `ichiran/dict:*identical-word-score-cutoff*`  — global, dict.lisp:1020
 468. `ichiran/dict:cull-segments`  — fn, dict.lisp:1027
 469. `ichiran/dict:gen-score`  — fn, dict.lisp:985
 470. `ichiran/dict:*force-kanji-break*`  — global, dict-errata.lisp:1226
 471. `ichiran/dict:*max-word-length*`  — global, dict.lisp:486
 472. `ichiran/dict:*no-kanji-break*`  — global, dict-errata.lisp:1229
 473. `ichiran/dict:*substring-hash*`  — global, dict.lisp:487
 474. `ichiran/dict:*suffix-map-temp*`  — global, dict.lisp:1049
 475. `ichiran/dict:*suffix-next-end*`  — global, dict.lisp:1050
 476. `ichiran/dict:find-sticky-positions`  — fn, dict.lisp:990
 477. `ichiran/dict:find-substring-words`  — fn, dict.lisp:501
 478. `ichiran/dict:verify`  — gf, dict-counters.lisp:0
 479. `ichiran/numbers:not-a-number`  — condition, numbers.lisp:0  *[ported]*
 480. `ichiran/dict:find-counter`  — fn, dict-counters.lisp:273
 481. `ichiran/dict:find-word`  — fn, dict.lisp:489
 482. `ichiran/dict:find-word-as-hiragana`  — fn, dict.lisp:592
 483. `ichiran/dict:adjoin-word`  — gf, dict.lisp:0
 484. `ichiran/dict:apply-patch`  — fn, dict-grammar.lisp:444
 485. `ichiran/dict:or-as-hiragana`  — fn, dict-grammar.lisp:95
 486. `ichiran/dict:suffix-ra`  — fn, dict-grammar.lisp:516
 487. `ichiran/dict:lex-compare`  — fn, dict-load.lisp:365
 488. `ichiran/dict:pair-words-by-conj`  — fn, dict-grammar.lisp:56
 489. `ichiran/dict:find-word-with-pos`  — fn, dict-grammar.lisp:87
 490. `ichiran/dict:suffix-suru`  — fn, dict-grammar.lisp:441
 491. `ichiran/dict:*suffix-unique-only*`  — global, dict-grammar.lisp:330
 492. `ichiran/dict:match-unique`  — fn, dict-grammar.lisp:702
 493. **CYCLE (49 symbols — port together)**
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
        - `ichiran/dict:find-word-full`  — fn, dict.lisp:1052
        - `ichiran/dict:find-word-suffix`  — fn, dict-grammar.lisp:706
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
 494. `ichiran/dict:get-suffix-map`  — fn, dict-grammar.lisp:685
 495. `ichiran/dict:make-segment`  — fn, dict.lisp:674
 496. `ichiran/dict:join-substring-words*`  — fn, dict.lisp:1069
 497. `ichiran/dict:make-segment-list`  — fn, dict.lisp:1038
 498. `ichiran/dict:join-substring-words`  — fn, dict.lisp:1113
 499. `ichiran/dict:dict-segment`  — fn, dict.lisp:1451
 500. `ichiran/dict:simple-segment`  — fn, dict.lisp:1456
 501. `ichiran/dict:gloss`  — dao, dict.lisp:178  *[ported]*
 502. `ichiran/dict:get-senses-raw`  — fn, dict.lisp:1458
 503. `ichiran/dict:get-senses`  — fn, dict.lisp:1487
 504. `ichiran/dict:get-senses-str`  — fn, dict.lisp:1495
 505. `ichiran/dict:*suffix-description*`  — global, dict-grammar.lisp:0
 506. `ichiran/dict:get-suffix-description`  — fn, dict-grammar.lisp:160
 507. `ichiran/dict:errata-conj-description-hook`  — fn, dict-errata.lisp:1320
 508. `ichiran/dict:load-conj-description`  — fn, dict-load.lisp:255
 509. `ichiran/dict:get-conj-description`  — fn, dict-load.lisp:255
 510. `ichiran/dict:conj-info-short`  — fn, dict.lisp:275
 511. `ichiran/dict:reading-str*`  — fn, dict.lisp:1580
 512. `ichiran/dict:reading-str-seq`  — fn, dict.lisp:1584
 513. `ichiran/dict:short-sense-str`  — fn, dict.lisp:1562
 514. `ichiran/dict:entry-info-short`  — fn, dict.lisp:1595
 515. `ichiran/dict:conj-type-order`  — fn, dict.lisp:1612
 516. `ichiran/dict:is-rareru`  — fn, dict.lisp:1619
 517. `ichiran/dict:filter-props`  — fn, dict.lisp:1627
 518. `ichiran/dict:select-conjs`  — fn, dict.lisp:1604
 519. `ichiran/dict:select-conjs-and-props`  — fn, dict.lisp:1640
 520. `ichiran/dict:print-conj-info`  — fn, dict.lisp:1649
 521. `ichiran/dict:query-parents-kana`  — fn, dict.lisp:415
 522. `ichiran/dict:best-kanji-conj`  — fn, dict.lisp:457
 523. `ichiran/dict:get-kanji`  — gf, dict.lisp:0
 524. `ichiran/dict:word-info-reading-str`  — fn, dict.lisp:1734
 525. `ichiran/dict:reading-str`  — gf, dict.lisp:0
 526. `ichiran/dict:word-info-str`  — fn, dict.lisp:1747
 527. `ichiran:*hepburn-kana-table*`  — global, romanize.lisp:0
 528. `ichiran:generic-romanization`  — class, romanize.lisp:62
 529. `ichiran:generic-hepburn`  — class, romanize.lisp:103
 530. `ichiran:simplified-hepburn`  — class, romanize.lisp:136
 531. `ichiran:traditional-hepburn`  — class, romanize.lisp:152
 532. `ichiran:*hepburn-traditional*`  — global, romanize.lisp:160
 533. `ichiran:*default-romanization-method*`  — global, romanize.lisp:203
 534. `ichiran:join-parts`  — fn, romanize.lisp:235
 535. `ichiran/dict:simplify-reading-list`  — fn, dict.lisp:1704
 536. `ichiran/dict:map-word-info-kana`  — fn, dict.lisp:1728
 537. `ichiran/dict:*hint-char-map*`  — global, dict-split.lisp:816
 538. `ichiran/dict:strip-hints`  — fn, dict-split.lisp:874
 539. `ichiran/dict:*kana-hint-mod*`  — global, dict-split.lisp:813
 540. `ichiran/dict:*hint-simplify-map*`  — global, dict-split.lisp:818
 541. `ichiran/dict:process-hints`  — fn, dict-split.lisp:872
 542. `ichiran:get-character-classes`  — fn, romanize.lisp:3
 543. `ichiran:r-special`  — gf, romanize.lisp:0
 544. `ichiran:process-iteration-characters`  — fn, romanize.lisp:7  *[ported]*
 545. `ichiran:process-modifiers`  — fn, romanize.lisp:15
 546. `ichiran:*kunrei-siki-kana-table*`  — global, romanize.lisp:0
 547. `ichiran:kunrei-siki`  — class, romanize.lisp:194
 548. `ichiran:r-simplify`  — gf, romanize.lisp:0
 549. `ichiran:leftmost-atom`  — fn, romanize.lisp:25
 550. `ichiran:r-base`  — gf, romanize.lisp:0
 551. **CYCLE (2 symbols — port together)**
        - `ichiran:r-apply`  — gf, romanize.lisp:0
        - `ichiran:romanize-core`  — fn, romanize.lisp:29
 552. `ichiran:romanize-list`  — fn, romanize.lisp:205
 553. `ichiran:romanize-word`  — fn, romanize.lisp:217
 554. `ichiran:romanize-word-info`  — fn, romanize.lisp:248
 555. `ichiran:romanize`  — fn, romanize.lisp:257
 556. `ichiran:romanize*`  — fn, romanize.lisp:273
 557. `ichiran/cli:main`  — fn, cli.lisp:48
 558. `ichiran/conn:*debug*`  — global, conn.lisp:20  *[skip — Debug-flag global gating dp. Replaced by the tracing crate's filter level.]*
 559. `ichiran/conn:def-conn-var`  — macro, conn.lisp:41  *[skip — Macro registering a global into the per-connection variable rebinding list. The cross-DB rebinding pattern is gone — each Ctx owns its caches directly.]*
 560. `ichiran/conn:defcache`  — macro, conn.lisp:135  *[skip — Macro registering a cache + defining init-cache method. Rust shape has no registry; each cache is a typed Ctx field with hand-written accessor.]*
 561. `ichiran/conn:dp`  — fn, conn.lisp:149  *[skip — Debug-printer wrapper around *debug*. Replaced by the tracing crate's emit + filter level.]*
 562. `ichiran/conn:let-db`  — macro, conn.lisp:32  *[skip — Rebinds *connection* for a dynamic scope. Multi-DB usage in Rust is Ctx::from_url(other); no scope-binding macro.]*
 563. `ichiran/conn:load-settings`  — fn, conn.lisp:76  *[skip — Loads settings.lisp and overrides connection from env. No counterpart in Rust — config comes from env (or layered config-crate sources) via Ctx::from_env.]*
 564. `ichiran/conn:with-db`  — macro, conn.lisp:46  *[skip — Rebinds *connection* and re-derives per-conn-var cache for a dynamic scope. Replaced by per-Ctx ownership of pool and caches; multi-DB = construct another Ctx.]*
 565. `ichiran/conn:with-log`  — macro, conn.lisp:86  *[skip — Wraps cl-postgres:*query-log* to a stream for the body. Replaced by sqlx + tracing query logging.]*
 566. `ichiran/custom:*municipality-types*`  — global, dict-custom.lisp:97
 567. `ichiran/custom:*municipality-types-description*`  — global, dict-custom.lisp:107
 568. `ichiran/custom:*municipality-types-order*`  — global, dict-custom.lisp:118
 569. `ichiran/custom:*silent-p*`  — global, dict-custom.lisp:5
 570. `ichiran/custom:as-xml-simple`  — fn, dict-custom.lisp:225
 571. `ichiran/custom:municipality`  — struct, dict-custom.lisp:140  *[ported]*
 572. `ichiran/custom:municipality-definition`  — fn, dict-custom.lisp:142
 573. `ichiran/custom:municipality-reading`  — fn, dict-custom.lisp:142
 574. `ichiran/custom:municipality-text`  — fn, dict-custom.lisp:142
 575. `ichiran/custom:ward`  — struct, dict-custom.lisp:269  *[ported]*
 576. `ichiran/custom:ward-definition`  — fn, dict-custom.lisp:274
 577. `ichiran/custom:ward-reading`  — fn, dict-custom.lisp:274
 578. `ichiran/custom:ward-text`  — fn, dict-custom.lisp:274
 579. `ichiran/custom:as-xml`  — gf, dict-custom.lisp:0
 580. `ichiran/custom:copy-municipality`  — fn, dict-custom.lisp:142
 581. `ichiran/custom:copy-ward`  — fn, dict-custom.lisp:274
 582. `ichiran/custom:xml-entry`  — struct, dict-custom.lisp:63  *[skip — XML reader out of scope per project decision (HANDOFF Resolved 2026-05-03); content slot holds a DOM document that cannot be constructed without an XML reader]*
 583. `ichiran/custom:copy-xml-entry`  — fn, dict-custom.lisp:61
 584. `ichiran/custom:custom-source`  — class, dict-custom.lisp:54
 585. `ichiran/custom:csv-loader`  — class, dict-custom.lisp:82
 586. `ichiran/custom:municipality-csv`  — class, dict-custom.lisp:93
 587. `ichiran/custom:source-path`  — fn, dict-custom.lisp:318
 588. `ichiran/custom:ward-csv`  — class, dict-custom.lisp:266
 589. `ichiran/custom:xml-loader`  — class, dict-custom.lisp:59
 590. `ichiran/custom:get-custom-data`  — fn, dict-custom.lisp:322
 591. `ichiran/custom:municipality-prefecture`  — fn, dict-custom.lisp:142
 592. `ichiran/custom:municipality-type`  — fn, dict-custom.lisp:142
 593. `ichiran/custom:ward-city`  — fn, dict-custom.lisp:274
 594. `ichiran/custom:get-words`  — gf, dict-custom.lisp:0
 595. `ichiran/dict:*pos-with-conj-rules*`  — global, dict-load.lisp:307
 596. `ichiran/dict:*do-not-conjugate*`  — global, dict-load.lisp:303
 597. `ichiran/dict:conjugation-rule`  — struct, dict-load.lisp:262  *[ported]*
 598. `ichiran/dict:cr-euphk`  — fn, dict-load.lisp:260
 599. `ichiran/dict:cr-euphr`  — fn, dict-load.lisp:260
 600. `ichiran/dict:cr-okuri`  — fn, dict-load.lisp:260
 601. `ichiran/dict:cr-stem`  — fn, dict-load.lisp:260
 602. `ichiran/dict:construct-conjugation`  — fn, dict-load.lisp:281
 603. `ichiran/dict:cr-conj`  — fn, dict-load.lisp:260
 604. `ichiran/dict:cr-fml`  — fn, dict-load.lisp:260
 605. `ichiran/dict:cr-neg`  — fn, dict-load.lisp:260
 606. `ichiran/dict:cr-onum`  — fn, dict-load.lisp:260
 607. `ichiran/dict:copy-conjugation-rule`  — fn, dict-load.lisp:260
 608. `ichiran/dict:load-pos-by-index`  — fn, dict-load.lisp:251
 609. `ichiran/dict:get-pos`  — fn, dict-load.lisp:251
 610. `ichiran/dict:load-pos-index`  — fn, dict-load.lisp:247
 611. `ichiran/dict:get-pos-index`  — fn, dict-load.lisp:247
 612. `ichiran/dict:make-conjugation-rule`  — fn, dict-load.lisp:260
 613. `ichiran/dict:errata-conj-rules-hook`  — fn, dict-errata.lisp:1329
 614. `ichiran/dict:load-conj-rules`  — fn, dict-load.lisp:265
 615. `ichiran/dict:get-conj-rules`  — fn, dict-load.lisp:265
 616. `ichiran/dict:conjugate-entry-inner`  — fn, dict-load.lisp:314
 617. `ichiran/dict:get-all-readings`  — fn, dict-errata.lisp:257
 618. `ichiran/dict:*secondary-conjugation-types-from*`  — global, dict-load.lisp:312
 619. `ichiran/dict:insert-conjugation`  — fn, dict-load.lisp:375
 620. `ichiran/dict:next-seq`  — fn, dict-load.lisp:110
 621. `ichiran/dict:conjugate-entry-outer`  — fn, dict-load.lisp:342
 622. `ichiran/dict:do-node-list-ord`  — macro, dict-load.lisp:26
 623. `ichiran/dict:node-text`  — fn, dict-load.lisp:14
 624. `ichiran/dict:restricted-readings`  — dao, dict.lisp:221  *[ported]*
 625. `ichiran/dict:insert-readings`  — fn, dict-load.lisp:32
 626. `ichiran/dict:insert-sense-traits`  — fn, dict-load.lisp:66
 627. `ichiran/dict:insert-senses`  — fn, dict-load.lisp:71
 628. `ichiran/dict:*secondary-conjugation-types*`  — global, dict-load.lisp:314
 629. `ichiran/dict:load-secondary-conjugations`  — fn, dict-load.lisp:457
 630. `ichiran/dict:load-entry`  — fn, dict-load.lisp:113
 631. `ichiran/custom:insert-entry`  — gf, dict-custom.lisp:0
 632. `ichiran/custom:normalize-geo`  — fn, dict-custom.lisp:176
 633. `ichiran/dict:get-candidates`  — fn, dict.lisp:1904
 634. `ichiran/dict:get-glosses`  — fn, dict.lisp:1892
 635. `ichiran/dict:match-glosses`  — fn, dict.lisp:1921
 636. `ichiran/custom:test-entry`  — gf, dict-custom.lisp:0
 637. `ichiran/dict:sense-exists-p`  — fn, dict-load.lisp:80
 638. `ichiran/dict:add-new-sense`  — fn, dict-load.lisp:91
 639. `ichiran/custom:update-entry`  — gf, dict-custom.lisp:0
 640. `ichiran/custom:update-entry-gloss`  — gf, dict-custom.lisp:0
 641. `ichiran/custom:xml-entry-content`  — fn, dict-custom.lisp:61
 642. `ichiran/custom:xml-entry-seq`  — fn, dict-custom.lisp:61
 643. `ichiran/custom:insert`  — gf, dict-custom.lisp:0
 644. `ichiran/custom:make-ward`  — fn, dict-custom.lisp:274
 645. `ichiran/custom:make-xml-entry`  — fn, dict-custom.lisp:61
 646. `ichiran/custom:make-municipality`  — fn, dict-custom.lisp:142
 647. `ichiran/custom:municipality-short`  — fn, dict-custom.lisp:123
 648. `ichiran:*hepburn-simple*`  — global, romanize.lisp:146
 649. `ichiran:romanize-word-geo`  — fn, romanize.lisp:232
 650. `ichiran/custom:romanize-municipality`  — fn, dict-custom.lisp:133
 651. `ichiran/custom:process-entry`  — gf, dict-custom.lisp:0
 652. `ichiran/custom:slurp`  — gf, dict-custom.lisp:0
 653. `ichiran/custom:load-custom-data`  — fn, dict-custom.lisp:329
 654. `ichiran/custom:municipality-p`  — fn, dict-custom.lisp:142
 655. `ichiran/custom:ward-p`  — fn, dict-custom.lisp:274
 656. `ichiran/custom:xml-entry-p`  — fn, dict-custom.lisp:61
 657. `ichiran/dict:*aux-verbs*`  — global, dict-grammar.lisp:1072
 658. `ichiran/dict:*conj-description*`  — global, dict-load.lisp:0
 659. `ichiran/dict:*conj-rules*`  — global, dict-load.lisp:0
 660. `ichiran/dict:*do-not-conjugate-seq*`  — global, dict-load.lisp:305
 661. `ichiran/dict:*easy-hints-seqs*`  — global, dict-split.lisp:904
 662. `ichiran/dict:*hints-checked*`  — global, dict-split.lisp:947
 663. `ichiran/dict:*honorifics*`  — global, dict-grammar.lisp:1156
 664. `ichiran/dict:*jmdict-data*`  — global, settings.lisp:12
 665. `ichiran/dict:*jmdict-path*`  — global, settings.lisp:10
 666. `ichiran/dict:*kana-hint-map*`  — global, dict-split.lisp:832
 667. `ichiran/dict:*noun-particles*`  — global, dict-grammar.lisp:801
 668. `ichiran/dict:*pos-by-index*`  — global, dict-load.lisp:0
 669. `ichiran/dict:*pos-index*`  — global, dict-load.lisp:0
 670. `ichiran/dict:find-conj`  — fn, dict-errata.lisp:1
 671. `ichiran/dict:add-conj`  — fn, dict-errata.lisp:15
 672. `ichiran/dict:root-diff`  — fn, dict-errata.lisp:95
 673. `ichiran/dict:root-diff-fn`  — fn, dict-errata.lisp:104
 674. `ichiran/dict:add-conj-reading`  — fn, dict-errata.lisp:109
 675. `ichiran/dict:add-reading`  — fn, dict-errata.lisp:35
 676. `ichiran/dict:add-deha-ja-readings`  — fn, dict-errata.lisp:171
 677. `ichiran/dict:add-sense-prop`  — fn, dict-errata.lisp:140
 678. `ichiran/dict:set-reading`  — gf, dict-load.lisp:0
 679. `ichiran/dict:reset-readings`  — fn, dict-errata.lisp:70
 680. `ichiran/dict:delete-reading`  — fn, dict-errata.lisp:76
 681. `ichiran/dict:set-common`  — fn, dict-errata.lisp:166
 682. `ichiran/dict:set-primary-nokanji`  — fn, dict-errata.lisp:224
 683. `ichiran/dict:add-errata-apr19`  — fn, dict-errata.lisp:847
 684. `ichiran/dict:add-new-sense*`  — fn, dict-errata.lisp:153
 685. `ichiran/dict:add-errata-apr20`  — fn, dict-errata.lisp:932
 686. `ichiran/dict:do-readings`  — macro, dict-errata.lisp:246
 687. `ichiran/dict:add-primary-nokanji`  — fn, dict-errata.lisp:251
 688. `ichiran/dict:delete-sense-prop`  — fn, dict-errata.lisp:136
 689. `ichiran/dict:add-errata-aug18`  — fn, dict-errata.lisp:803
 690. `ichiran/dict:add-gloss`  — fn, dict-errata.lisp:156
 691. `ichiran/dict:add-errata-counters`  — fn, dict-errata.lisp:1159
 692. `ichiran/dict:add-errata-dec23`  — fn, dict-errata.lisp:1028
 693. `ichiran/dict:add-errata-feb17`  — fn, dict-errata.lisp:608
 694. `ichiran/dict:add-errata-jan18`  — fn, dict-errata.lisp:697
 695. `ichiran/dict:add-errata-jan19`  — fn, dict-errata.lisp:823
 696. `ichiran/dict:add-errata-jan20`  — fn, dict-errata.lisp:867
 697. `ichiran/dict:replace-reading`  — fn, dict-errata.lisp:49
 698. `ichiran/dict:add-errata-jan21`  — fn, dict-errata.lisp:979
 699. `ichiran/dict:add-errata-jan22`  — fn, dict-errata.lisp:1017
 700. `ichiran/dict:replace-reading-conj`  — fn, dict-errata.lisp:60
 701. `ichiran/dict:add-errata-jan25`  — fn, dict-errata.lisp:1055
 702. `ichiran/dict:add-errata-jan26`  — fn, dict-errata.lisp:1077
 703. `ichiran/dict:rearrange-readings`  — fn, dict-errata.lisp:229
 704. `ichiran/dict:rearrange-readings-conj`  — fn, dict-errata.lisp:241
 705. `ichiran/dict:add-errata-jul20`  — fn, dict-errata.lisp:961
 706. `ichiran/dict:add-errata-mar18`  — fn, dict-errata.lisp:764
 707. `ichiran/dict:add-errata-may21`  — fn, dict-errata.lisp:1006
 708. `ichiran/dict:delete-conjugation`  — fn, dict-errata.lisp:198
 709. `ichiran/dict:add-gozaimasu-conjs`  — fn, dict-errata.lisp:263
 710. `ichiran/dict:conjugate-da`  — fn, dict-errata.lisp:281
 711. `ichiran/dict:delete-senses`  — fn, dict-errata.lisp:129
 712. `ichiran/dict:remove-hiragana-nokanji`  — fn, dict-errata.lisp:217
 713. `ichiran/dict:add-errata`  — fn, dict-errata.lisp:289
 714. `ichiran/dict:add-sense`  — fn, dict-errata.lisp:146
 715. `ichiran/dict:true-kana`  — gf, dict.lisp:0
 716. `ichiran/dict:true-kanji`  — gf, dict.lisp:0
 717. `ichiran/kanji:reading`  — dao, kanji.lisp:42  *[ported]*
 718. `ichiran/kanji:get-reading-alternatives`  — fn, kanji.lisp:216
 719. `ichiran/kanji:*reading-cache*`  — global, kanji.lisp:199
 720. `ichiran/kanji:kanji`  — dao, kanji.lisp:10  *[ported]*
 721. `ichiran/kanji:get-readings-cache`  — fn, kanji.lisp:199
 722. `ichiran/kanji:get-normal-readings`  — fn, kanji.lisp:231
 723. `ichiran/kanji:make-rmap`  — fn, kanji.lisp:273
 724. `ichiran/kanji:match-readings*`  — fn, kanji.lisp:241
 725. `ichiran/kanji:match-readings`  — fn, kanji.lisp:292
 726. `ichiran/dict:check-easy-hints`  — fn, dict-split.lisp:950
 727. `ichiran/dict:conj-data-p`  — fn, dict.lisp:325
 728. `ichiran/dict:conj-data-seq`  — fn, dict.lisp:325
 729. `ichiran/dict:conj-prop-json`  — fn, dict.lisp:283
 730. `ichiran/dict:find-words-seqs`  — fn, dict.lisp:520
 731. `ichiran/dict:get-original-text-once`  — fn, dict.lisp:369
 732. `ichiran/dict:match-kana-kanji`  — fn, dict.lisp:1507
 733. `ichiran/dict:match-sense-restrictions`  — fn, dict.lisp:1515
 734. `ichiran/dict:split-pos`  — fn, dict.lisp:1535
 735. `ichiran/dict:get-senses-json`  — fn, dict.lisp:1537
 736. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:conj-info-json`  — fn, dict.lisp:1698
        - `ichiran/dict:conj-info-json*`  — fn, dict.lisp:1665
 737. `ichiran/dict:conjugate-word`  — fn, dict-load.lisp:294
 738. `ichiran/dict:conjugation-rule-p`  — fn, dict-load.lisp:260
 739. `ichiran/dict:copy-conj-data`  — fn, dict.lisp:325
 740. `ichiran/dict:copy-synergy`  — fn, dict-grammar.lisp:727
 741. `ichiran/dict:copy-top-array-item`  — fn, dict.lisp:1138
 742. `ichiran/dict:cr-pos`  — fn, dict-load.lisp:260
 743. `ichiran/dict:csv-hash`  — macro, dict-load.lisp:201
 744. `ichiran/dict:defsuffix`  — macro, dict-grammar.lisp:342
 745. `ichiran/dict:def-abbr-suffix`  — macro, dict-grammar.lisp:557
 746. `ichiran/dict:defsplit`  — macro, dict-split.lisp:5
 747. `ichiran/dict:def-simple-split`  — macro, dict-split.lisp:11
 748. `ichiran/dict:def-de-split`  — macro, dict-split.lisp:81
 749. `ichiran/dict:def-do-split`  — macro, dict-split.lisp:181
 750. `ichiran/dict:defhint`  — macro, dict-split.lisp:892
 751. `ichiran/dict:insert-hints`  — fn, dict-split.lisp:875
 752. `ichiran/dict:translate-hint-position`  — fn, dict-split.lisp:930
 753. `ichiran/dict:translate-hints`  — fn, dict-split.lisp:942
 754. `ichiran/dict:def-easy-hint`  — macro, dict-split.lisp:955
 755. `ichiran/dict:defpenalty`  — macro, dict-grammar.lisp:981
 756. `ichiran/dict:def-generic-penalty`  — macro, dict-grammar.lisp:984
 757. `ichiran/dict:defsynergy`  — macro, dict-grammar.lisp:738
 758. `ichiran/dict:def-generic-synergy`  — macro, dict-grammar.lisp:739
 759. `ichiran/dict:def-reader-for-json`  — macro, dict.lisp:1289
 760. `ichiran/dict:defsegfilter`  — macro, dict-grammar.lisp:1043
 761. `ichiran/dict:def-segfilter-must-follow`  — macro, dict-grammar.lisp:1049
 762. `ichiran/dict:def-shi-split`  — macro, dict-split.lisp:191
 763. `ichiran/dict:def-simple-hint`  — macro, dict-split.lisp:901
 764. `ichiran/dict:def-simple-suffix`  — macro, dict-grammar.lisp:345
 765. `ichiran/dict:def-special-counter`  — macro, dict-counters.lisp:361
 766. `ichiran/dict:def-toori-split`  — macro, dict-split.lisp:143
 767. `ichiran/dict:delete-duplicate-props`  — fn, dict.lisp:295
 768. `ichiran/dict:drop-extras`  — fn, dict-load.lisp:194
 769. `ichiran/dict:entry-digest`  — fn, dict.lisp:64
 770. `ichiran/dict:entry-info-long`  — fn, dict.lisp:1601
 771. `ichiran/dict:exists-reading`  — fn, dict.lisp:1847
 772. `ichiran/dict:filter-is-pos`  — macro, dict-grammar.lisp:772
 773. `ichiran/dict:find-word-kana-pattern`  — fn, dict.lisp:1877
 774. `ichiran/dict:find-kanji-for-pattern`  — fn, dict.lisp:1882
 775. `ichiran/dict:find-word-info`  — fn, dict.lisp:1850
 776. `ichiran/dict:word-info-reading`  — fn, dict.lisp:1445
 777. `ichiran/dict:word-info-gloss-json`  — fn, dict.lisp:1784
 778. `ichiran/dict:find-word-info-json`  — fn, dict.lisp:1872
 779. `ichiran/dict:fix-entities`  — fn, dict-load.lisp:159
 780. `ichiran/dict:get-kanji-words`  — fn, dict.lisp:1836
 781. `ichiran/dict:init-tables`  — fn, dict-load.lisp:3
 782. `ichiran/dict:length-multiplier`  — fn, dict.lisp:681
 783. `ichiran/dict:load-best-readings`  — fn, dict-load.lisp:530
 784. `ichiran/dict:load-conjugations`  — fn, dict-load.lisp:445
 785. `ichiran/dict:recalc-entry-stats-all`  — fn, dict.lisp:59
 786. `ichiran/dict:load-extras`  — fn, dict-load.lisp:183
 787. `ichiran/dict:load-jmdict`  — fn, dict-load.lisp:168
 788. `ichiran/dict:recalc-entry-stats`  — fn, dict.lisp:53
 789. `ichiran/dict:segment-list-p`  — fn, dict.lisp:1038
 790. `ichiran/dict:segment-p`  — fn, dict.lisp:674
 791. `ichiran/dict:segment-top`  — fn, dict.lisp:674
 792. `ichiran/dict:word-info-json`  — fn, dict.lisp:1262
 793. `ichiran/dict:simple-word-info`  — fn, dict.lisp:1282
 794. `ichiran/dict:split-kigatsuku`  — fn, dict-split.lisp:298
 795. `ichiran/dict:substring-index`  — fn, dict.lisp:1132
 796. `ichiran/dict:suffix-sou-base`  — macro, dict-grammar.lisp:445
 797. `ichiran/dict:synergy-connector`  — fn, dict-grammar.lisp:727
 798. `ichiran/dict:synergy-description`  — fn, dict-grammar.lisp:727
 799. `ichiran/dict:synergy-end`  — fn, dict-grammar.lisp:727
 800. `ichiran/dict:synergy-p`  — fn, dict-grammar.lisp:727
 801. `ichiran/dict:synergy-start`  — fn, dict-grammar.lisp:727
 802. `ichiran/dict:top-array-item-p`  — fn, dict.lisp:1138
 803. `ichiran/dict:word-info-from-text`  — fn, dict.lisp:1382
 804. `ichiran/dict:word-info-rec-find`  — fn, dict.lisp:1409
 805. `ichiran/dict:word-readings`  — fn, dict.lisp:536
 806. `ichiran/kanji:*kanjidic-path*`  — global, settings.lisp:16
 807. `ichiran/kanji:calculate-perc`  — fn, kanji.lisp:349
 808. `ichiran/kanji:first-node-text`  — fn, kanji.lisp:106
 809. `ichiran/kanji:get-original-reading`  — fn, kanji.lisp:308
 810. `ichiran/kanji:get-reading-stats`  — fn, kanji.lisp:399
 811. `ichiran/kanji:get-readings`  — fn, kanji.lisp:211
 812. `ichiran/kanji:meaning`  — dao, kanji.lisp:83  *[ported]*
 813. `ichiran/kanji:okurigana`  — dao, kanji.lisp:67  *[ported]*
 814. `ichiran/kanji:init-tables`  — fn, kanji.lisp:98
 815. `ichiran:*hepburn-basic*`  — global, romanize.lisp:144
 816. `ichiran/kanji:reading-info-json`  — fn, kanji.lisp:354
 817. `ichiran/kanji:to-json`  — gf, kanji.lisp:0
 818. `ichiran/kanji:kanji-info-json`  — fn, kanji.lisp:392
 819. `ichiran/kanji:kanji-reading-json`  — fn, kanji.lisp:410
 820. `ichiran/kanji:kanji-word-stats`  — fn, kanji.lisp:316
 821. `ichiran/kanji:load-readings`  — fn, kanji.lisp:114
 822. `ichiran/kanji:load-kanji`  — fn, kanji.lisp:152
 823. `ichiran/kanji:load-kanji-stats`  — fn, kanji.lisp:332
 824. `ichiran/kanji:load-kanjidic`  — fn, kanji.lisp:185
 825. `ichiran/kanji:process-match-json`  — fn, kanji.lisp:428
 826. `ichiran/kanji:match-readings-json`  — fn, kanji.lisp:452
 827. `ichiran/kanji:query-kanji-json`  — macro, kanji.lisp:458
 828. `ichiran/numbers:*digit-kanji-legal*`  — global, numbers.lisp:5  *[ported]*
 829. `ichiran/numbers:parse-number*`  — fn, numbers.lisp:57  *[ported]*
 830. `ichiran/numbers:parse-number`  — fn, numbers.lisp:77  *[ported]*
 831. `ichiran:modified-hepburn`  — class, romanize.lisp:162
 832. `ichiran:*hepburn-modified*`  — global, romanize.lisp:168
 833. `ichiran:*hepburn-passport*`  — global, romanize.lisp:149
 834. `ichiran:*kunrei-siki*`  — global, romanize.lisp:201
 835. `ichiran:rmap-item`  — struct, deromanize.lisp:5  *[ported]*
 836. `ichiran:*romaji-kana*`  — global, deromanize.lisp:0
 837. `ichiran:has-successors`  — fn, deromanize.lisp:11
 838. `ichiran:*romaji-kana-next*`  — global, deromanize.lisp:21
 839. `ichiran:kana-representation`  — struct, deromanize.lisp:23  *[ported]*
 840. `ichiran:make-kana-representation`  — fn, deromanize.lisp:21
 841. `ichiran:possible-long-vowel-p`  — fn, deromanize.lisp:30
 842. `ichiran:rmi-kana`  — fn, deromanize.lisp:3
 843. `ichiran:rmi-next`  — fn, deromanize.lisp:3
 844. `ichiran:rmi-text`  — fn, deromanize.lisp:3
 845. `ichiran:apply-rmap-item`  — fn, deromanize.lisp:35
 846. `ichiran:kr-branch`  — fn, deromanize.lisp:21
 847. `ichiran:kr-canonical`  — fn, deromanize.lisp:21
 848. `ichiran:kr-pattern`  — fn, deromanize.lisp:21
 849. `ichiran:kr-rest`  — fn, deromanize.lisp:21
 850. `ichiran:join-branches`  — fn, deromanize.lisp:54
 851. `ichiran:kr-concat`  — fn, deromanize.lisp:23
 852. `ichiran:make-rmap-item`  — fn, deromanize.lisp:3
 853. `ichiran:load-romaji-kana`  — fn, deromanize.lisp:5
 854. `ichiran:get-romaji-kana`  — fn, deromanize.lisp:5
 855. `ichiran:romaji-next`  — fn, deromanize.lisp:46
 856. `ichiran:branches-next`  — fn, deromanize.lisp:69
 857. `ichiran:copy-kana-representation`  — fn, deromanize.lisp:21
 858. `ichiran:copy-rmap-item`  — fn, deromanize.lisp:3
 859. `ichiran:kana-representation-p`  — fn, deromanize.lisp:21
 860. `ichiran:rmap-item-p`  — fn, deromanize.lisp:3
 861. `ichiran:romaji-kana`  — fn, deromanize.lisp:84
 862. `ichiran:romaji-suggest`  — fn, deromanize.lisp:95
