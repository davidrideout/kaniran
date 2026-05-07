# Port plan — 837 symbols in 779 waves (7 mutual-recursion groups covering 65 symbols)
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
  77. `ichiran/dict:simple-text`  — class, dict.lisp:69  *[ported]*
  78. `ichiran/dict:kana-text`  — dao, dict.lisp:128  *[ported]*
  79. `ichiran/dict:counter-text`  — class, dict-counters.lisp:9  *[ported]*
  80. `ichiran/dict:counter-age`  — class, dict-counters.lisp:757  *[ported]*
  81. `ichiran/dict:counter-days-kun`  — class, dict-counters.lisp:686  *[ported]*
  82. `ichiran/dict:counter-days-on`  — class, dict-counters.lisp:709  *[ported]*
  83. `ichiran/dict:counter-halfhour`  — class, dict-counters.lisp:391  *[ported]*
  84. `ichiran/dict:counter-hifumi`  — class, dict-counters.lisp:518  *[ported]*
  85. `ichiran/dict:counter-months`  — class, dict-counters.lisp:721  *[ported]*
  86. `ichiran/dict:counter-people`  — class, dict-counters.lisp:735  *[ported]*
  87. `ichiran/dict:counter-tsu`  — class, dict-counters.lisp:497  *[ported]*
  88. `ichiran/dict:counter-wari`  — class, dict-counters.lisp:746  *[ported]*
  89. `ichiran/dict:*special-counters*`  — global, dict-counters.lisp:211  *[ported]*
  90. `ichiran/dict:conjugation`  — dao, dict.lisp:238  *[ported]*
  91. `ichiran/dict:*extra-counter-ids*`  — global, dict-counters.lisp:310  *[ported]*
  92. `ichiran/dict:*skip-counter-ids*`  — global, dict-counters.lisp:315  *[ported]*
  93. `ichiran/dict:sense-prop`  — dao, dict.lisp:197  *[ported]*
  94. `ichiran/dict:get-counter-ids`  — fn, dict-counters.lisp:285  *[ported]*
  95. `ichiran/dict:get-counter-stags`  — fn, dict-counters.lisp:292  *[ported]*
  96. `ichiran/dict:gloss`  — dao, dict.lisp:178  *[ported]*
  97. `ichiran/dict:kanji-text`  — dao, dict.lisp:86  *[ported]*
  98. `ichiran/dict:get-counter-readings`  — fn, dict-counters.lisp:335  *[ported]*
  99. `ichiran/dict:sense`  — dao, dict.lisp:166  *[ported]*
 100. `ichiran/dict:entry`  — dao, dict.lisp:26  *[ported]*
 101. `ichiran/dict:no-conj-data`  — fn, dict.lisp:337  *[ported]*
 102. `ichiran/dict:*suffix-cache*`  — global, dict-grammar.lisp:0  *[wip — empty-map stub: populated by wave 127 init-suffixes via init-suffix-hashtables + load-kf + every def-simple-suffix callsite. Replace with proper init when wave 127 lands.]*
 103. `ichiran/dict:*suffix-class*`  — global, dict-grammar.lisp:0  *[wip — empty-map stub: populated by wave 127 init-suffixes via init-suffix-hashtables + load-kf + every def-simple-suffix callsite. Replace with proper init when wave 127 lands.]*
 104. **CYCLE (4 symbols — port together)**
        - `ichiran/conn:*conn-vars*`  — global, conn.lisp:39  *[skip — Registry of per-connection-rebound globals. Unneeded once each Ctx owns its caches directly.]*
        - `ichiran/dict:*counter-cache*`  — global, dict-counters.lisp:0  *[ported]*
        - `ichiran/dict:*is-arch-cache*`  — global, dict.lisp:0  *[ported]*
        - `ichiran/dict:*no-conj-data*`  — global, dict.lisp:0  *[ported]*
 105. `ichiran/conn:*connections*`  — global, settings.lisp:5  *[skip — Alist of secondary connection specs. Replaced by call-site Ctx::from_url(...) per database; no global registry.]*
 106. `ichiran/conn:get-spec`  — fn, conn.lisp:25  *[skip — Lisp dbid-dispatch (nil/list/keyword → connection spec) doesn't translate. Connection registry will be handled via the Rust config crate when the DB layer lands.]*
 107. `ichiran/conn:switch-conn-vars`  — fn, conn.lisp:65  *[skip — Per-connection variable rebinding from *conn-var-cache*. Rust has no dynamic-variable shadowing; replaced by per-Database struct ownership of caches when the DB layer lands. Same family as all-caches / get-spec.]*
 108. `ichiran/dict:init-suffix-hashtables`  — fn, dict-grammar.lisp:6  *[skip — Empty-hashtable initializer for *suffix-cache* / *suffix-class* def-conn-vars. Rust replacement is OnceLock<HashMap> populated on first read; no standalone init verb survives.]*
 109. `ichiran/dict:*init-suffixes-lock*`  — global, dict-grammar.lisp:163  *[skip — SBCL mutex guarding init-suffixes-thread's populator and powering init-suffixes-running-p. Subsumed by OnceLock::get_or_init's built-in once-only synchronization on *suffix-cache* / *suffix-class*; no standalone mutex survives.]*
 110. `ichiran/dict:init-suffixes-running-p`  — fn, dict-grammar.lisp:165  *[skip — Loader-busy predicate over a one-shot init thread + def-conn-var cache. Rust replacement is OnceLock::get().is_some() or eager startup init; the verb has nowhere to live.]*
 111. `ichiran/dict:conj-prop`  — dao, dict.lisp:262  *[ported]*
 112. `ichiran/dict:find-word-seq`  — fn, dict-grammar.lisp:73  *[ported]*
 113. `ichiran/dict:find-word-conj-of`  — fn, dict-grammar.lisp:77  *[ported]*
 114. `ichiran/dict:get-kana-form`  — fn, dict-grammar.lisp:36  *[ported]*
 115. `ichiran/dict:conj-data`  — struct, dict.lisp:327  *[ported]*
 116. `ichiran/dict:conj-source-reading`  — dao, dict.lisp:309  *[ported]*
 117. `ichiran/dict:get-conj-data`  — fn, dict.lisp:340  *[ported]*
 118. `ichiran/dict:*weak-conj-forms*`  — global, dict-errata.lisp:1316  *[ported]*
 119. `ichiran/dict:*skip-conj-forms*`  — global, dict-errata.lisp:1310  *[ported]*
 120. `ichiran/dict:test-conj-prop`  — fn, dict-errata.lisp:1336  *[ported]*
 121. `ichiran/dict:skip-by-conj-data`  — fn, dict-errata.lisp:1336  *[ported]*  *[extracted: tatoeba]*  *[audited 33168/33168]*
 122. `ichiran/dict:get-kana-forms-conj-data-filter`  — fn, dict-grammar.lisp:10  *[ported]*  *[extracted: init-suffixes]*
 123. `ichiran/dict:get-kana-forms*`  — fn, dict-grammar.lisp:17  *[ported]*  *[extracted: init-suffixes]*
 124. `ichiran/dict:get-kana-forms`  — fn, dict-grammar.lisp:32  *[ported]*  *[extracted: init-suffixes]*
 125. `ichiran/dict:init-suffixes-thread`  — fn, dict-grammar.lisp:169  *[ported]*
 126. `ichiran/dict:init-suffixes`  — fn, dict-grammar.lisp:332  *[skip — Subsumed by KaniranContext eager construction; init-suffixes-thread (wave 126) is the actual populator. Same prior-art as init-suffixes-running-p.]*
 127. `ichiran/cli:build`  — fn, cli.lisp:102  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 128. `ichiran/cli:print-romanize-info`  — fn, cli.lisp:44  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 129. `ichiran/cli:unknown-option`  — fn, cli.lisp:33  *[skip — CLI-only entrypoint/help glue; belongs in a future kaniran-cli crate]*
 130. `ichiran/conn:*is-dynamic-connection*`  — global, conn.lisp:14  *[skip — "Boolean marking 'connection came from env]*
 131. `ichiran/conn:*connection-env-var*`  — global, conn.lisp:13  *[ported]*
 132. `ichiran/conn:get-ichiran-connection-env`  — fn, conn.lisp:154  *[ported]*
 133. `ichiran/conn:load-connection-from-env`  — fn, conn.lisp:166  *[skip — "Side-effects-on-globals semantics (set *connection*]*
 134. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:compound-text`  — class, dict.lisp:608  *[ported]*
        - `ichiran/dict:score-base`  — gf, dict.lisp:0
 135. `ichiran/dict:proxy-text`  — class, dict.lisp:550  *[ported]*
 136. `ichiran/dict:true-text`  — gf, dict.lisp:0
 137. `ichiran/dict:word-info`  — class, dict.lisp:1245  *[ported]*
 138. `ichiran/dict:process-word-info`  — fn, dict.lisp:1417  *[ported]*
 139. `ichiran/dict:synergy`  — struct, dict-grammar.lisp:713  *[ported]*
 140. **CYCLE (4 symbols — port together)**
        - `ichiran/dict:segment`  — struct, dict.lisp:674
        - `ichiran/dict:segment-list`  — struct, dict.lisp:1038
        - `ichiran/dict:top-array`  — class, dict.lisp:1140
        - `ichiran/dict:top-array-item`  — struct, dict.lisp:1138
 141. `ichiran/dict:*segment-score-cutoff*`  — global, dict.lisp:1351
 142. `ichiran/dict:get-text`  — gf, dict.lisp:0
 143. `ichiran/dict:ordinal-str`  — fn, dict-counters.lisp:38
 144. `ichiran/dict:value-string`  — gf, dict-counters.lisp:0
 145. `ichiran/dict:word-type`  — gf, dict.lisp:0
 146. `ichiran/dict:word-info-from-segment`  — fn, dict.lisp:1327
 147. `ichiran/dict:word-info-from-segment-list`  — fn, dict.lisp:1353
 148. `ichiran/dict:fill-segment-path`  — fn, dict.lisp:1390
 149. `ichiran/dict:split-1010105`  — fn, dict-split.lisp:771
 150. `ichiran/dict:split-1567610`  — fn, dict-split.lisp:771
 151. `ichiran/dict:split-1675330`  — fn, dict-split.lisp:771
 152. `ichiran/dict:split-2841254`  — fn, dict-split.lisp:771
 153. `ichiran/dict:split-dakara`  — fn, dict-split.lisp:771
 154. `ichiran/dict:split-deha`  — fn, dict-split.lisp:771
 155. `ichiran/dict:split-dokoroka`  — fn, dict-split.lisp:771
 156. `ichiran/dict:split-hitorashii`  — fn, dict-split.lisp:771
 157. `ichiran/dict:split-honno`  — fn, dict-split.lisp:771
 158. `ichiran/dict:split-kanatte`  — fn, dict-split.lisp:771
 159. `ichiran/dict:split-naito`  — fn, dict-split.lisp:771
 160. `ichiran/dict:split-omise`  — fn, dict-split.lisp:771
 161. `ichiran/dict:split-toha`  — fn, dict-split.lisp:771
 162. `ichiran/dict:split-tokorode`  — fn, dict-split.lisp:771
 163. `ichiran/dict:split-tokorodewa`  — fn, dict-split.lisp:771
 164. `ichiran/dict:split-tokoroe`  — fn, dict-split.lisp:771
 165. `ichiran/dict:split-tokoroga`  — fn, dict-split.lisp:771
 166. `ichiran/dict:split-tokorowo`  — fn, dict-split.lisp:771
 167. `ichiran/dict:*segsplit-map*`  — global, dict-split.lisp:704
 168. `ichiran/dict:split-1000430`  — fn, dict-split.lisp:505
 169. `ichiran/dict:split-1002970`  — fn, dict-split.lisp:492
 170. `ichiran/dict:split-1005600`  — fn, dict-split.lisp:498
 171. `ichiran/dict:split-1006280`  — fn, dict-split.lisp:669
 172. `ichiran/dict:split-1006880`  — fn, dict-split.lisp:727
 173. `ichiran/dict:split-1008030`  — fn, dict-split.lisp:645
 174. `ichiran/dict:split-1207840`  — fn, dict-split.lisp:711
 175. `ichiran/dict:split-1221530`  — fn, dict-split.lisp:611
 176. `ichiran/dict:split-1221680`  — fn, dict-split.lisp:521
 177. `ichiran/dict:split-1314600`  — fn, dict-split.lisp:512
 178. `ichiran/dict:split-1314770`  — fn, dict-split.lisp:640
 179. `ichiran/dict:split-1315860`  — fn, dict-split.lisp:535
 180. `ichiran/dict:split-1322540`  — fn, dict-split.lisp:517
 181. `ichiran/dict:split-1322560`  — fn, dict-split.lisp:719
 182. `ichiran/dict:split-1327220`  — fn, dict-split.lisp:424
 183. `ichiran/dict:split-1327230`  — fn, dict-split.lisp:429
 184. `ichiran/dict:split-1349300`  — fn, dict-split.lisp:608
 185. `ichiran/dict:split-1362970`  — fn, dict-split.lisp:759
 186. `ichiran/dict:split-1474200`  — fn, dict-split.lisp:546
 187. `ichiran/dict:split-1502500`  — fn, dict-split.lisp:487
 188. `ichiran/dict:split-1508380`  — fn, dict-split.lisp:478
 189. `ichiran/dict:split-1532270`  — fn, dict-split.lisp:685
 190. `ichiran/dict:split-1538340`  — fn, dict-split.lisp:526
 191. `ichiran/dict:split-1551500`  — fn, dict-split.lisp:631
 192. `ichiran/dict:split-1579130`  — fn, dict-split.lisp:559
 193. `ichiran/dict:split-1581550`  — fn, dict-split.lisp:650
 194. `ichiran/dict:split-1591050`  — fn, dict-split.lisp:571
 195. `ichiran/dict:split-1591980`  — fn, dict-split.lisp:625
 196. `ichiran/dict:split-1597740`  — fn, dict-split.lisp:645
 197. `ichiran/dict:split-1601010`  — fn, dict-split.lisp:732
 198. `ichiran/dict:split-1601080`  — fn, dict-split.lisp:658
 199. `ichiran/dict:split-1602740`  — fn, dict-split.lisp:605
 200. `ichiran/dict:split-1606530`  — fn, dict-split.lisp:676
 201. `ichiran/dict:split-1606800`  — fn, dict-split.lisp:706
 202. `ichiran/dict:split-1612640`  — fn, dict-split.lisp:509
 203. `ichiran/dict:split-1774820`  — fn, dict-split.lisp:756
 204. `ichiran/dict:split-1854750`  — fn, dict-split.lisp:596
 205. `ichiran/dict:split-1855670`  — fn, dict-split.lisp:742
 206. `ichiran/dict:split-1863230`  — fn, dict-split.lisp:698
 207. `ichiran/dict:split-1881690`  — fn, dict-split.lisp:734
 208. `ichiran/dict:optprefix`  — fn, dict-split.lisp:580
 209. `ichiran/dict:split-1894260`  — fn, dict-split.lisp:586
 210. `ichiran/dict:split-2002270`  — fn, dict-split.lisp:633
 211. `ichiran/dict:split-2007500`  — fn, dict-split.lisp:681
 212. `ichiran/dict:split-2009290`  — fn, dict-split.lisp:483
 213. `ichiran/dict:split-2016840`  — fn, dict-split.lisp:502
 214. `ichiran/dict:split-2026650`  — fn, dict-split.lisp:601
 215. `ichiran/dict:split-2083990`  — fn, dict-split.lisp:468
 216. `ichiran/dict:split-2088480`  — fn, dict-split.lisp:438
 217. `ichiran/dict:split-2109610`  — fn, dict-split.lisp:715
 218. `ichiran/dict:split-2133750`  — fn, dict-split.lisp:691
 219. `ichiran/dict:split-2272780`  — fn, dict-split.lisp:616
 220. `ichiran/dict:split-2276360`  — fn, dict-split.lisp:554
 221. `ichiran/dict:split-2433760`  — fn, dict-split.lisp:432
 222. `ichiran/dict:split-2526850`  — fn, dict-split.lisp:597
 223. `ichiran/dict:split-2529050`  — fn, dict-split.lisp:662
 224. `ichiran/dict:split-2666360`  — fn, dict-split.lisp:446
 225. `ichiran/dict:split-2668400`  — fn, dict-split.lisp:564
 226. `ichiran/dict:split-2724560`  — fn, dict-split.lisp:442
 227. `ichiran/dict:split-2757500`  — fn, dict-split.lisp:531
 228. `ichiran/dict:split-2757540`  — fn, dict-split.lisp:673
 229. `ichiran/dict:split-2762260`  — fn, dict-split.lisp:474
 230. `ichiran/dict:split-2771940`  — fn, dict-split.lisp:457
 231. `ichiran/dict:split-2834051`  — fn, dict-split.lisp:702
 232. `ichiran/dict:split-2834732`  — fn, dict-split.lisp:740
 233. `ichiran/dict:split-2835890`  — fn, dict-split.lisp:577
 234. `ichiran/dict:split-2846470`  — fn, dict-split.lisp:621
 235. `ichiran/dict:split-2855921`  — fn, dict-split.lisp:748
 236. `ichiran/dict:split-de-1004800`  — fn, dict-split.lisp:104
 237. `ichiran/dict:split-de-1006840`  — fn, dict-split.lisp:106
 238. `ichiran/dict:split-de-1163700`  — fn, dict-split.lisp:102
 239. `ichiran/dict:split-de-1189420`  — fn, dict-split.lisp:111
 240. `ichiran/dict:split-de-1245390`  — fn, dict-split.lisp:108
 241. `ichiran/dict:split-de-1270210`  — fn, dict-split.lisp:140
 242. `ichiran/dict:split-de-1272220`  — fn, dict-split.lisp:112
 243. `ichiran/dict:split-de-1311360`  — fn, dict-split.lisp:113
 244. `ichiran/dict:split-de-1343110`  — fn, dict-split.lisp:139
 245. `ichiran/dict:split-de-1368500`  — fn, dict-split.lisp:114
 246. `ichiran/dict:split-de-1395670`  — fn, dict-split.lisp:115
 247. `ichiran/dict:split-de-1417790`  — fn, dict-split.lisp:116
 248. `ichiran/dict:split-de-1454270`  — fn, dict-split.lisp:117
 249. `ichiran/dict:split-de-1479100`  — fn, dict-split.lisp:119
 250. `ichiran/dict:split-de-1510140`  — fn, dict-split.lisp:120
 251. `ichiran/dict:split-de-1518550`  — fn, dict-split.lisp:121
 252. `ichiran/dict:split-de-1530610`  — fn, dict-split.lisp:107
 253. `ichiran/dict:split-de-1531420`  — fn, dict-split.lisp:122
 254. `ichiran/dict:split-de-1597400`  — fn, dict-split.lisp:123
 255. `ichiran/dict:split-de-1611020`  — fn, dict-split.lisp:102
 256. `ichiran/dict:split-de-1679990`  — fn, dict-split.lisp:124
 257. `ichiran/dict:split-de-1682060`  — fn, dict-split.lisp:126
 258. `ichiran/dict:split-de-1736650`  — fn, dict-split.lisp:127
 259. `ichiran/dict:split-de-1865020`  — fn, dict-split.lisp:128
 260. `ichiran/dict:split-de-1878880`  — fn, dict-split.lisp:129
 261. `ichiran/dict:split-de-2126220`  — fn, dict-split.lisp:130
 262. `ichiran/dict:split-de-2136520`  — fn, dict-split.lisp:131
 263. `ichiran/dict:split-de-2513590`  — fn, dict-split.lisp:133
 264. `ichiran/dict:split-de-2719270`  — fn, dict-split.lisp:109
 265. `ichiran/dict:split-de-2771850`  — fn, dict-split.lisp:135
 266. `ichiran/dict:split-de-2810720`  — fn, dict-split.lisp:105
 267. `ichiran/dict:split-de-2810800`  — fn, dict-split.lisp:136
 268. `ichiran/dict:split-degozaimasu`  — fn, dict-split.lisp:140
 269. `ichiran/dict:split-desura`  — fn, dict-split.lisp:382
 270. `ichiran/dict:split-do-2142680`  — fn, dict-split.lisp:190
 271. `ichiran/dict:split-do-2142710`  — fn, dict-split.lisp:189
 272. `ichiran/dict:split-do-2523480`  — fn, dict-split.lisp:190
 273. `ichiran/dict:split-do-2803190`  — fn, dict-split.lisp:189
 274. `ichiran/dict:split-dogatsukeru`  — fn, dict-split.lisp:276
 275. `ichiran/dict:split-gotoni`  — fn, dict-split.lisp:387
 276. `ichiran/dict:split-hairikomeru`  — fn, dict-split.lisp:340
 277. `ichiran/dict:split-hajiketobu`  — fn, dict-split.lisp:328
 278. `ichiran/dict:split-hajikidasu`  — fn, dict-split.lisp:368
 279. `ichiran/dict:split-hayaimonode`  — fn, dict-split.lisp:267
 280. `ichiran/dict:split-hisshininatte`  — fn, dict-split.lisp:348
 281. `ichiran/dict:split-hitotachi`  — fn, dict-split.lisp:375
 282. `ichiran/dict:split-jan`  — fn, dict-split.lisp:454
 283. `ichiran/dict:split-janai`  — fn, dict-split.lisp:449
 284. `ichiran/dict:split-janaika`  — fn, dict-split.lisp:281
 285. `ichiran/dict:split-kaasan`  — fn, dict-split.lisp:285
 286. `ichiran/dict:split-kaisasae`  — fn, dict-split.lisp:399
 287. `ichiran/dict:split-katawonaraberu`  — fn, dict-split.lisp:305
 288. `ichiran/dict:split-kawaribae`  — fn, dict-split.lisp:258
 289. `ichiran/dict:split-kimatte`  — fn, dict-split.lisp:314
 290. `ichiran/dict:split-kinosei`  — fn, dict-split.lisp:295
 291. `ichiran/dict:split-kotonisuru`  — fn, dict-split.lisp:360
 292. `ichiran/dict:split-motteiku`  — fn, dict-split.lisp:333
 293. `ichiran/dict:split-moushiwakenasasou`  — fn, dict-split.lisp:310
 294. `ichiran/dict:split-nakunaru`  — fn, dict-split.lisp:237
 295. `ichiran/dict:split-nakunaru2`  — fn, dict-split.lisp:244
 296. `ichiran/dict:split-nanimokamo`  — fn, dict-split.lisp:301
 297. `ichiran/dict:split-nantokanaru`  — fn, dict-split.lisp:323
 298. `ichiran/dict:split-nara`  — fn, dict-split.lisp:464
 299. `ichiran/dict:split-nitotte`  — fn, dict-split.lisp:354
 300. `ichiran/dict:split-osagari`  — fn, dict-split.lisp:395
 301. `ichiran/dict:split-osoreiru`  — fn, dict-split.lisp:318
 302. `ichiran/dict:split-shi-1005700`  — fn, dict-split.lisp:209
 303. `ichiran/dict:split-shi-1005830`  — fn, dict-split.lisp:210
 304. `ichiran/dict:split-shi-1157200`  — fn, dict-split.lisp:211
 305. `ichiran/dict:split-shi-1157220`  — fn, dict-split.lisp:212
 306. `ichiran/dict:split-shi-1157230`  — fn, dict-split.lisp:213
 307. `ichiran/dict:split-shi-1157240`  — fn, dict-split.lisp:232
 308. `ichiran/dict:split-shi-1157280`  — fn, dict-split.lisp:214
 309. `ichiran/dict:split-shi-1157310`  — fn, dict-split.lisp:215
 310. `ichiran/dict:split-shi-1304820`  — fn, dict-split.lisp:234
 311. `ichiran/dict:split-shi-1304890`  — fn, dict-split.lisp:216
 312. `ichiran/dict:split-shi-1304960`  — fn, dict-split.lisp:218
 313. `ichiran/dict:split-shi-1305110`  — fn, dict-split.lisp:219
 314. `ichiran/dict:split-shi-1305280`  — fn, dict-split.lisp:221
 315. `ichiran/dict:split-shi-1305290`  — fn, dict-split.lisp:223
 316. `ichiran/dict:split-shi-1594300`  — fn, dict-split.lisp:223
 317. `ichiran/dict:split-shi-1594310`  — fn, dict-split.lisp:225
 318. `ichiran/dict:split-shi-1594460`  — fn, dict-split.lisp:227
 319. `ichiran/dict:split-shi-1594580`  — fn, dict-split.lisp:228
 320. `ichiran/dict:split-shi-2518250`  — fn, dict-split.lisp:231
 321. `ichiran/dict:split-shi-2858937`  — fn, dict-split.lisp:235
 322. `ichiran/dict:split-shinikakaru`  — fn, dict-split.lisp:345
 323. `ichiran/dict:split-souda`  — fn, dict-split.lisp:290
 324. `ichiran/dict:split-soudesu`  — fn, dict-split.lisp:292
 325. `ichiran/dict:split-tegakakaru`  — fn, dict-split.lisp:249
 326. `ichiran/dict:split-toiu`  — fn, dict-split.lisp:404
 327. `ichiran/dict:split-toiukotoda`  — fn, dict-split.lisp:407
 328. `ichiran/dict:split-tonaru`  — fn, dict-split.lisp:419
 329. `ichiran/dict:split-tonattara`  — fn, dict-split.lisp:415
 330. `ichiran/dict:split-toori-1164910`  — fn, dict-split.lisp:174
 331. `ichiran/dict:split-toori-1260990`  — fn, dict-split.lisp:155
 332. `ichiran/dict:split-toori-1368820`  — fn, dict-split.lisp:171
 333. `ichiran/dict:split-toori-1414570`  — fn, dict-split.lisp:157
 334. `ichiran/dict:split-toori-1424950`  — fn, dict-split.lisp:159
 335. `ichiran/dict:split-toori-1424960`  — fn, dict-split.lisp:161
 336. `ichiran/dict:split-toori-1462720`  — fn, dict-split.lisp:179
 337. `ichiran/dict:split-toori-1489800`  — fn, dict-split.lisp:167
 338. `ichiran/dict:split-toori-1523010`  — fn, dict-split.lisp:169
 339. `ichiran/dict:split-toori-1550490`  — fn, dict-split.lisp:172
 340. `ichiran/dict:split-toori-1619440`  — fn, dict-split.lisp:173
 341. `ichiran/dict:split-toori-1808080`  — fn, dict-split.lisp:171
 342. `ichiran/dict:split-toori-1820790`  — fn, dict-split.lisp:165
 343. `ichiran/dict:*split-map*`  — global, dict-split.lisp:5
 344. `ichiran/dict:*copulae*`  — global, dict-errata.lisp:1205
 345. `ichiran/dict:*final-prt*`  — global, dict-errata.lisp:1182
 346. `ichiran/dict:*non-final-prt*`  — global, dict-errata.lisp:1209
 347. `ichiran/dict:*semi-final-prt*`  — global, dict-errata.lisp:1196
 348. `ichiran/dict:*skip-words*`  — global, dict-errata.lisp:1155
 349. `ichiran/dict:apply-score-mod`  — gf, dict.lisp:0
 350. `ichiran/dict:compare-common`  — fn, dict.lisp:1022
 351. `ichiran/dict:get-non-arch-posi`  — fn, dict.lisp:762
 352. `ichiran/dict:get-original-text*`  — fn, dict.lisp:378
 353. `ichiran/dict:restricted-readings`  — dao, dict.lisp:221  *[ported]*
 354. `ichiran/dict:word-conj-data`  — gf, dict.lisp:0
 355. `ichiran/dict:get-original-text`  — gf, dict.lisp:0
 356. `ichiran/dict:get-split*`  — fn, dict-split.lisp:67
 357. `ichiran/dict:get-split`  — fn, dict-split.lisp:75
 358. `ichiran/dict:is-arch`  — fn, dict.lisp:760  *[ported]*
 359. `ichiran/dict:*no-kanji-break-penalty*`  — global, dict-errata.lisp:1214
 360. `ichiran/dict:*score-cutoff*`  — global, dict.lisp:1069
 361. `ichiran/dict:parse-suffix-val`  — fn, dict-grammar.lisp:679
 362. `ichiran/dict:make-slice`  — fn, dict.lisp:1010
 363. `ichiran/dict:subseq-slice`  — fn, dict.lisp:1013
 364. `ichiran/dict:get-suffixes`  — fn, dict-grammar.lisp:697
 365. `ichiran/dict:*length-coeff-sequences*`  — global, dict.lisp:686
 366. `ichiran/dict:length-multiplier-coeff`  — fn, dict.lisp:694
 367. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:calc-score`  — fn, dict.lisp:775
        - `ichiran/dict:kanji-break-penalty`  — fn, dict.lisp:702
 368. `ichiran/dict:get-segsplit`  — fn, dict-split.lisp:823
 369. `ichiran/dict:expand-segment-list`  — fn, dict.lisp:1180
 370. `ichiran/dict:*gap-penalty*`  — global, dict.lisp:1165
 371. `ichiran/dict:gap-penalty`  — fn, dict.lisp:1169
 372. `ichiran/dict:get-array`  — gf, dict.lisp:0
 373. `ichiran/dict:classify`  — fn, dict-grammar.lisp:1046
 374. `ichiran/dict:filter-in-seq-set`  — fn, dict-grammar.lisp:783
 375. `ichiran/dict:filter-is-conjugation`  — fn, dict-grammar.lisp:797
 376. `ichiran/dict:make-segment-list-from`  — fn, dict-grammar.lisp:733
 377. `ichiran/dict:segfilter-aux-verb`  — fn, dict-grammar.lisp:1099
 378. `ichiran/dict:filter-is-compound-end-text`  — fn, dict-grammar.lisp:820
 379. `ichiran/dict:segfilter-badend`  — fn, dict-grammar.lisp:1114
 380. `ichiran/dict:segfilter-dashi`  — fn, dict-grammar.lisp:1167
 381. `ichiran/dict:segfilter-dekiru`  — fn, dict-grammar.lisp:1175
 382. `ichiran/dict:segfilter-honorific`  — fn, dict-grammar.lisp:1177
 383. `ichiran/dict:filter-is-compound-end`  — fn, dict-grammar.lisp:806
 384. `ichiran/dict:segfilter-janai`  — fn, dict-grammar.lisp:1146
 385. `ichiran/dict:segfilter-mononi`  — fn, dict-grammar.lisp:1177
 386. `ichiran/dict:filter-in-seq-set-simple`  — fn, dict-grammar.lisp:787
 387. `ichiran/dict:segfilter-n`  — fn, dict-grammar.lisp:1106
 388. `ichiran/dict:segfilter-nohayamete`  — fn, dict-grammar.lisp:1151
 389. `ichiran/dict:segfilter-roku`  — fn, dict-grammar.lisp:1129
 390. `ichiran/dict:segfilter-sae`  — fn, dict-grammar.lisp:1141
 391. `ichiran/dict:segfilter-sukiyoki`  — fn, dict-grammar.lisp:1119
 392. `ichiran/dict:segfilter-toomou`  — fn, dict-grammar.lisp:1156
 393. `ichiran/dict:segfilter-totte`  — fn, dict-grammar.lisp:1165
 394. `ichiran/dict:segfilter-tsu-iru`  — fn, dict-grammar.lisp:1101
 395. `ichiran/dict:segfilter-wokarasu`  — fn, dict-grammar.lisp:1112
 396. `ichiran/dict:*segfilter-list*`  — global, dict-grammar.lisp:1024
 397. `ichiran/dict:apply-segfilters`  — fn, dict-grammar.lisp:1177
 398. `ichiran/dict:get-seg-initial`  — fn, dict.lisp:1172
 399. `ichiran/dict:penalty-semi-final`  — fn, dict-grammar.lisp:1022
 400. `ichiran/dict:filter-short-kana`  — fn, dict-grammar.lisp:1008
 401. `ichiran/dict:penalty-short`  — fn, dict-grammar.lisp:1020
 402. `ichiran/dict:*penalty-list*`  — global, dict-grammar.lisp:964
 403. `ichiran/dict:get-penalties`  — fn, dict-grammar.lisp:1030
 404. `ichiran/dict:synergy-kanji-prefix`  — fn, dict-grammar.lisp:940
 405. `ichiran/dict:synergy-na-adjectives`  — fn, dict-grammar.lisp:892
 406. `ichiran/dict:synergy-no-adjectives`  — fn, dict-grammar.lisp:884
 407. `ichiran/dict:synergy-no-da`  — fn, dict-grammar.lisp:871
 408. `ichiran/dict:synergy-no-toori`  — fn, dict-grammar.lisp:970
 409. `ichiran/dict:filter-is-noun`  — fn, dict-grammar.lisp:760
 410. `ichiran/dict:synergy-noun-da`  — fn, dict-grammar.lisp:859
 411. `ichiran/dict:synergy-noun-particle`  — fn, dict-grammar.lisp:850
 412. `ichiran/dict:synergy-o-prefix`  — fn, dict-grammar.lisp:935
 413. `ichiran/dict:synergy-oki`  — fn, dict-grammar.lisp:973
 414. `ichiran/dict:synergy-shicha-ikenai`  — fn, dict-grammar.lisp:951
 415. `ichiran/dict:synergy-shika-negative`  — fn, dict-grammar.lisp:959
 416. `ichiran/dict:synergy-sou-nanda`  — fn, dict-grammar.lisp:878
 417. `ichiran/dict:synergy-suffix-buri`  — fn, dict-grammar.lisp:925
 418. `ichiran/dict:synergy-suffix-chu`  — fn, dict-grammar.lisp:914
 419. `ichiran/dict:synergy-suffix-sei`  — fn, dict-grammar.lisp:929
 420. `ichiran/dict:synergy-suffix-tachi`  — fn, dict-grammar.lisp:921
 421. `ichiran/dict:synergy-to-adverbs`  — fn, dict-grammar.lisp:902
 422. `ichiran/dict:*synergy-list*`  — global, dict-grammar.lisp:723
 423. `ichiran/dict:get-synergies`  — fn, dict-grammar.lisp:976
 424. `ichiran/dict:get-seg-splits`  — fn, dict.lisp:1175
 425. `ichiran/dict:get-segment-score`  — gf, dict.lisp:0
 426. `ichiran/dict:register-item`  — gf, dict.lisp:0
 427. `ichiran/dict:find-best-path`  — fn, dict.lisp:1190
 428. `ichiran/dict:*identical-word-score-cutoff*`  — global, dict.lisp:1020
 429. `ichiran/dict:cull-segments`  — fn, dict.lisp:1027
 430. `ichiran/dict:gen-score`  — fn, dict.lisp:985
 431. `ichiran/dict:*force-kanji-break*`  — global, dict-errata.lisp:1226
 432. `ichiran/dict:*max-word-length*`  — global, dict.lisp:486
 433. `ichiran/dict:*no-kanji-break*`  — global, dict-errata.lisp:1229
 434. `ichiran/dict:*substring-hash*`  — global, dict.lisp:487
 435. `ichiran/dict:*suffix-map-temp*`  — global, dict.lisp:1049
 436. `ichiran/dict:*suffix-next-end*`  — global, dict.lisp:1050
 437. `ichiran/dict:find-sticky-positions`  — fn, dict.lisp:990
 438. `ichiran/dict:find-substring-words`  — fn, dict.lisp:501
 439. `ichiran/dict:verify`  — gf, dict-counters.lisp:0
 440. `ichiran/numbers:not-a-number`  — condition, numbers.lisp:0  *[ported]*
 441. `ichiran/dict:find-counter`  — fn, dict-counters.lisp:273
 442. `ichiran/dict:find-word`  — fn, dict.lisp:489
 443. `ichiran/dict:find-word-as-hiragana`  — fn, dict.lisp:592
 444. `ichiran/dict:adjoin-word`  — gf, dict.lisp:0
 445. `ichiran/dict:apply-patch`  — fn, dict-grammar.lisp:444
 446. `ichiran/dict:or-as-hiragana`  — fn, dict-grammar.lisp:95
 447. `ichiran/dict:suffix-ra`  — fn, dict-grammar.lisp:516
 448. `ichiran/dict:lex-compare`  — fn, dict-load.lisp:365
 449. `ichiran/dict:pair-words-by-conj`  — fn, dict-grammar.lisp:56
 450. `ichiran/dict:find-word-with-pos`  — fn, dict-grammar.lisp:87
 451. `ichiran/dict:suffix-suru`  — fn, dict-grammar.lisp:441
 452. `ichiran/dict:*suffix-unique-only*`  — global, dict-grammar.lisp:330
 453. `ichiran/dict:match-unique`  — fn, dict-grammar.lisp:702
 454. **CYCLE (49 symbols — port together)**
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
 455. `ichiran/dict:get-suffix-map`  — fn, dict-grammar.lisp:685
 456. `ichiran/dict:join-substring-words*`  — fn, dict.lisp:1069
 457. `ichiran/dict:join-substring-words`  — fn, dict.lisp:1113
 458. `ichiran/dict:dict-segment`  — fn, dict.lisp:1451
 459. `ichiran/dict:simple-segment`  — fn, dict.lisp:1456
 460. `ichiran/dict:get-senses-raw`  — fn, dict.lisp:1458
 461. `ichiran/dict:get-senses`  — fn, dict.lisp:1487
 462. `ichiran/dict:get-senses-str`  — fn, dict.lisp:1495
 463. `ichiran/dict:*suffix-description*`  — global, dict-grammar.lisp:0
 464. `ichiran/dict:get-suffix-description`  — fn, dict-grammar.lisp:160
 465. `ichiran/dict:errata-conj-description-hook`  — fn, dict-errata.lisp:1320
 466. `ichiran/dict:load-conj-description`  — fn, dict-load.lisp:255
 467. `ichiran/dict:get-conj-description`  — fn, dict-load.lisp:255
 468. `ichiran/dict:conj-info-short`  — fn, dict.lisp:275
 469. `ichiran/dict:reading-str*`  — fn, dict.lisp:1580
 470. `ichiran/dict:reading-str-seq`  — fn, dict.lisp:1584
 471. `ichiran/dict:short-sense-str`  — fn, dict.lisp:1562
 472. `ichiran/dict:entry-info-short`  — fn, dict.lisp:1595
 473. `ichiran/dict:conj-type-order`  — fn, dict.lisp:1612
 474. `ichiran/dict:is-rareru`  — fn, dict.lisp:1619
 475. `ichiran/dict:filter-props`  — fn, dict.lisp:1627
 476. `ichiran/dict:select-conjs`  — fn, dict.lisp:1604
 477. `ichiran/dict:select-conjs-and-props`  — fn, dict.lisp:1640
 478. `ichiran/dict:print-conj-info`  — fn, dict.lisp:1649
 479. `ichiran/dict:query-parents-kana`  — fn, dict.lisp:415
 480. `ichiran/dict:best-kanji-conj`  — fn, dict.lisp:457
 481. `ichiran/numbers:*digit-kanji-default*`  — global, numbers.lisp:3  *[ported]*
 482. `ichiran/numbers:*power-kanji*`  — global, numbers.lisp:7  *[ported]*
 483. `ichiran/numbers:number-to-kanji`  — fn, numbers.lisp:35  *[ported]*
 484. `ichiran/dict:get-kanji`  — gf, dict.lisp:0
 485. `ichiran/dict:word-info-reading-str`  — fn, dict.lisp:1734
 486. `ichiran/dict:reading-str`  — gf, dict.lisp:0
 487. `ichiran/dict:word-info-str`  — fn, dict.lisp:1747
 488. `ichiran:*hepburn-kana-table*`  — global, romanize.lisp:0
 489. `ichiran:generic-romanization`  — class, romanize.lisp:62
 490. `ichiran:generic-hepburn`  — class, romanize.lisp:103
 491. `ichiran:simplified-hepburn`  — class, romanize.lisp:136
 492. `ichiran:traditional-hepburn`  — class, romanize.lisp:152
 493. `ichiran:*hepburn-traditional*`  — global, romanize.lisp:160
 494. `ichiran:*default-romanization-method*`  — global, romanize.lisp:203
 495. `ichiran:join-parts`  — fn, romanize.lisp:235
 496. `ichiran/dict:simplify-reading-list`  — fn, dict.lisp:1704
 497. `ichiran/dict:map-word-info-kana`  — fn, dict.lisp:1728
 498. `ichiran/dict:*hint-char-map*`  — global, dict-split.lisp:816
 499. `ichiran/dict:strip-hints`  — fn, dict-split.lisp:874
 500. `ichiran/dict:*kana-hint-mod*`  — global, dict-split.lisp:813
 501. `ichiran/dict:*kana-hint-space*`  — global, dict-split.lisp:814  *[ported]*
 502. `ichiran/dict:*hint-simplify-map*`  — global, dict-split.lisp:818
 503. `ichiran/dict:process-hints`  — fn, dict-split.lisp:872
 504. `ichiran:get-character-classes`  — fn, romanize.lisp:3
 505. `ichiran:r-special`  — gf, romanize.lisp:0
 506. `ichiran:process-iteration-characters`  — fn, romanize.lisp:7  *[ported]*
 507. `ichiran:process-modifiers`  — fn, romanize.lisp:15
 508. `ichiran:*kunrei-siki-kana-table*`  — global, romanize.lisp:0
 509. `ichiran:kunrei-siki`  — class, romanize.lisp:194
 510. `ichiran:r-simplify`  — gf, romanize.lisp:0
 511. `ichiran:leftmost-atom`  — fn, romanize.lisp:25
 512. `ichiran:r-base`  — gf, romanize.lisp:0
 513. **CYCLE (2 symbols — port together)**
        - `ichiran:r-apply`  — gf, romanize.lisp:0
        - `ichiran:romanize-core`  — fn, romanize.lisp:29
 514. `ichiran:romanize-list`  — fn, romanize.lisp:205
 515. `ichiran:romanize-word`  — fn, romanize.lisp:217
 516. `ichiran:romanize-word-info`  — fn, romanize.lisp:248
 517. `ichiran:romanize`  — fn, romanize.lisp:257
 518. `ichiran:romanize*`  — fn, romanize.lisp:273
 519. `ichiran/cli:main`  — fn, cli.lisp:48
 520. `ichiran/conn:*debug*`  — global, conn.lisp:20  *[skip — Debug-flag global gating dp. Replaced by the tracing crate's filter level.]*
 521. `ichiran/conn:def-conn-var`  — macro, conn.lisp:41  *[skip — Macro registering a global into the per-connection variable rebinding list. The cross-DB rebinding pattern is gone — each Ctx owns its caches directly.]*
 522. `ichiran/conn:defcache`  — macro, conn.lisp:135  *[skip — Macro registering a cache + defining init-cache method. Rust shape has no registry; each cache is a typed Ctx field with hand-written accessor.]*
 523. `ichiran/conn:dp`  — fn, conn.lisp:149  *[skip — Debug-printer wrapper around *debug*. Replaced by the tracing crate's emit + filter level.]*
 524. `ichiran/conn:let-db`  — macro, conn.lisp:32  *[skip — Rebinds *connection* for a dynamic scope. Multi-DB usage in Rust is Ctx::from_url(other); no scope-binding macro.]*
 525. `ichiran/conn:load-settings`  — fn, conn.lisp:76  *[skip — Loads settings.lisp and overrides connection from env. No counterpart in Rust — config comes from env (or layered config-crate sources) via Ctx::from_env.]*
 526. `ichiran/conn:with-db`  — macro, conn.lisp:46  *[skip — Rebinds *connection* and re-derives per-conn-var cache for a dynamic scope. Replaced by per-Ctx ownership of pool and caches; multi-DB = construct another Ctx.]*
 527. `ichiran/conn:with-log`  — macro, conn.lisp:86  *[skip — Wraps cl-postgres:*query-log* to a stream for the body. Replaced by sqlx + tracing query logging.]*
 528. `ichiran/custom:*municipality-types*`  — global, dict-custom.lisp:97
 529. `ichiran/custom:*municipality-types-description*`  — global, dict-custom.lisp:107
 530. `ichiran/custom:*municipality-types-order*`  — global, dict-custom.lisp:118
 531. `ichiran/custom:*silent-p*`  — global, dict-custom.lisp:5
 532. `ichiran/custom:as-xml-simple`  — fn, dict-custom.lisp:225
 533. `ichiran/custom:municipality`  — struct, dict-custom.lisp:140  *[ported]*
 534. `ichiran/custom:ward`  — struct, dict-custom.lisp:269  *[ported]*
 535. `ichiran/custom:as-xml`  — gf, dict-custom.lisp:0
 536. `ichiran/custom:custom-source`  — class, dict-custom.lisp:54
 537. `ichiran/custom:csv-loader`  — class, dict-custom.lisp:82
 538. `ichiran/custom:municipality-csv`  — class, dict-custom.lisp:93
 539. `ichiran/custom:source-path`  — fn, dict-custom.lisp:318
 540. `ichiran/custom:ward-csv`  — class, dict-custom.lisp:266
 541. `ichiran/custom:xml-loader`  — class, dict-custom.lisp:59
 542. `ichiran/custom:get-custom-data`  — fn, dict-custom.lisp:322
 543. `ichiran/custom:get-words`  — gf, dict-custom.lisp:0
 544. `ichiran/dict:*pos-with-conj-rules*`  — global, dict-load.lisp:307
 545. `ichiran/dict:*do-not-conjugate*`  — global, dict-load.lisp:303
 546. `ichiran/dict:conjugation-rule`  — struct, dict-load.lisp:262  *[ported]*
 547. `ichiran/dict:construct-conjugation`  — fn, dict-load.lisp:281
 548. `ichiran/dict:load-pos-by-index`  — fn, dict-load.lisp:251
 549. `ichiran/dict:get-pos`  — fn, dict-load.lisp:251
 550. `ichiran/dict:load-pos-index`  — fn, dict-load.lisp:247
 551. `ichiran/dict:get-pos-index`  — fn, dict-load.lisp:247
 552. `ichiran/dict:errata-conj-rules-hook`  — fn, dict-errata.lisp:1329
 553. `ichiran/dict:load-conj-rules`  — fn, dict-load.lisp:265
 554. `ichiran/dict:get-conj-rules`  — fn, dict-load.lisp:265
 555. `ichiran/dict:conjugate-entry-inner`  — fn, dict-load.lisp:314
 556. `ichiran/dict:get-all-readings`  — fn, dict-errata.lisp:257
 557. `ichiran/dict:*secondary-conjugation-types-from*`  — global, dict-load.lisp:312
 558. `ichiran/dict:insert-conjugation`  — fn, dict-load.lisp:375
 559. `ichiran/dict:next-seq`  — fn, dict-load.lisp:110
 560. `ichiran/dict:conjugate-entry-outer`  — fn, dict-load.lisp:342
 561. `ichiran/dict:do-node-list-ord`  — macro, dict-load.lisp:26
 562. `ichiran/dict:node-text`  — fn, dict-load.lisp:14
 563. `ichiran/dict:insert-readings`  — fn, dict-load.lisp:32
 564. `ichiran/dict:insert-sense-traits`  — fn, dict-load.lisp:66
 565. `ichiran/dict:insert-senses`  — fn, dict-load.lisp:71
 566. `ichiran/dict:*secondary-conjugation-types*`  — global, dict-load.lisp:314
 567. `ichiran/dict:load-secondary-conjugations`  — fn, dict-load.lisp:457
 568. `ichiran/dict:load-entry`  — fn, dict-load.lisp:113
 569. `ichiran/custom:insert-entry`  — gf, dict-custom.lisp:0
 570. `ichiran/custom:normalize-geo`  — fn, dict-custom.lisp:176
 571. `ichiran/dict:get-candidates`  — fn, dict.lisp:1904
 572. `ichiran/dict:get-glosses`  — fn, dict.lisp:1892
 573. `ichiran/dict:match-glosses`  — fn, dict.lisp:1921
 574. `ichiran/custom:test-entry`  — gf, dict-custom.lisp:0
 575. `ichiran/dict:sense-exists-p`  — fn, dict-load.lisp:80
 576. `ichiran/dict:add-new-sense`  — fn, dict-load.lisp:91
 577. `ichiran/custom:update-entry`  — gf, dict-custom.lisp:0
 578. `ichiran/custom:update-entry-gloss`  — gf, dict-custom.lisp:0
 579. `ichiran/custom:xml-entry`  — struct, dict-custom.lisp:63  *[skip — XML reader out of scope per project decision (HANDOFF Resolved 2026-05-03); content slot holds a DOM document that cannot be constructed without an XML reader]*
 580. `ichiran/custom:insert`  — gf, dict-custom.lisp:0
 581. `ichiran/custom:municipality-short`  — fn, dict-custom.lisp:123
 582. `ichiran:*hepburn-simple*`  — global, romanize.lisp:146
 583. `ichiran:romanize-word-geo`  — fn, romanize.lisp:232
 584. `ichiran/custom:romanize-municipality`  — fn, dict-custom.lisp:133
 585. `ichiran/custom:process-entry`  — gf, dict-custom.lisp:0
 586. `ichiran/custom:slurp`  — gf, dict-custom.lisp:0
 587. `ichiran/custom:load-custom-data`  — fn, dict-custom.lisp:329
 588. `ichiran/dict:*aux-verbs*`  — global, dict-grammar.lisp:1072
 589. `ichiran/dict:*conj-description*`  — global, dict-load.lisp:0
 590. `ichiran/dict:*conj-rules*`  — global, dict-load.lisp:0
 591. `ichiran/dict:*disable-hints*`  — global, dict.lisp:78  *[ported]*
 592. `ichiran/dict:*do-not-conjugate-seq*`  — global, dict-load.lisp:305
 593. `ichiran/dict:*easy-hints-seqs*`  — global, dict-split.lisp:904
 594. `ichiran/dict:*hint-map*`  — global, dict-split.lisp:850
 595. `ichiran/dict:*hints-checked*`  — global, dict-split.lisp:947
 596. `ichiran/dict:*honorifics*`  — global, dict-grammar.lisp:1156
 597. `ichiran/dict:*jmdict-data*`  — global, settings.lisp:12
 598. `ichiran/dict:*jmdict-path*`  — global, settings.lisp:10
 599. `ichiran/dict:*kana-hint-map*`  — global, dict-split.lisp:832
 600. `ichiran/dict:*noun-particles*`  — global, dict-grammar.lisp:801
 601. `ichiran/dict:*pos-by-index*`  — global, dict-load.lisp:0
 602. `ichiran/dict:*pos-index*`  — global, dict-load.lisp:0
 603. `ichiran/dict:find-conj`  — fn, dict-errata.lisp:1
 604. `ichiran/dict:add-conj`  — fn, dict-errata.lisp:15
 605. `ichiran/dict:root-diff`  — fn, dict-errata.lisp:95
 606. `ichiran/dict:root-diff-fn`  — fn, dict-errata.lisp:104
 607. `ichiran/dict:add-conj-reading`  — fn, dict-errata.lisp:109
 608. `ichiran/dict:add-reading`  — fn, dict-errata.lisp:35
 609. `ichiran/dict:add-deha-ja-readings`  — fn, dict-errata.lisp:171
 610. `ichiran/dict:add-sense-prop`  — fn, dict-errata.lisp:140
 611. `ichiran/dict:set-reading`  — gf, dict-load.lisp:0
 612. `ichiran/dict:reset-readings`  — fn, dict-errata.lisp:70
 613. `ichiran/dict:delete-reading`  — fn, dict-errata.lisp:76
 614. `ichiran/dict:set-common`  — fn, dict-errata.lisp:166
 615. `ichiran/dict:set-primary-nokanji`  — fn, dict-errata.lisp:224
 616. `ichiran/dict:add-errata-apr19`  — fn, dict-errata.lisp:847
 617. `ichiran/dict:add-new-sense*`  — fn, dict-errata.lisp:153
 618. `ichiran/dict:add-errata-apr20`  — fn, dict-errata.lisp:932
 619. `ichiran/dict:do-readings`  — macro, dict-errata.lisp:246
 620. `ichiran/dict:add-primary-nokanji`  — fn, dict-errata.lisp:251
 621. `ichiran/dict:delete-sense-prop`  — fn, dict-errata.lisp:136
 622. `ichiran/dict:add-errata-aug18`  — fn, dict-errata.lisp:803
 623. `ichiran/dict:add-gloss`  — fn, dict-errata.lisp:156
 624. `ichiran/dict:add-errata-counters`  — fn, dict-errata.lisp:1159
 625. `ichiran/dict:add-errata-dec23`  — fn, dict-errata.lisp:1028
 626. `ichiran/dict:add-errata-feb17`  — fn, dict-errata.lisp:608
 627. `ichiran/dict:add-errata-jan18`  — fn, dict-errata.lisp:697
 628. `ichiran/dict:add-errata-jan19`  — fn, dict-errata.lisp:823
 629. `ichiran/dict:add-errata-jan20`  — fn, dict-errata.lisp:867
 630. `ichiran/dict:replace-reading`  — fn, dict-errata.lisp:49
 631. `ichiran/dict:add-errata-jan21`  — fn, dict-errata.lisp:979
 632. `ichiran/dict:add-errata-jan22`  — fn, dict-errata.lisp:1017
 633. `ichiran/dict:replace-reading-conj`  — fn, dict-errata.lisp:60
 634. `ichiran/dict:add-errata-jan25`  — fn, dict-errata.lisp:1055
 635. `ichiran/dict:add-errata-jan26`  — fn, dict-errata.lisp:1077
 636. `ichiran/dict:rearrange-readings`  — fn, dict-errata.lisp:229
 637. `ichiran/dict:rearrange-readings-conj`  — fn, dict-errata.lisp:241
 638. `ichiran/dict:add-errata-jul20`  — fn, dict-errata.lisp:961
 639. `ichiran/dict:add-errata-mar18`  — fn, dict-errata.lisp:764
 640. `ichiran/dict:add-errata-may21`  — fn, dict-errata.lisp:1006
 641. `ichiran/dict:delete-conjugation`  — fn, dict-errata.lisp:198
 642. `ichiran/dict:add-gozaimasu-conjs`  — fn, dict-errata.lisp:263
 643. `ichiran/dict:conjugate-da`  — fn, dict-errata.lisp:281
 644. `ichiran/dict:delete-senses`  — fn, dict-errata.lisp:129
 645. `ichiran/dict:remove-hiragana-nokanji`  — fn, dict-errata.lisp:217
 646. `ichiran/dict:add-errata`  — fn, dict-errata.lisp:289
 647. `ichiran/dict:add-sense`  — fn, dict-errata.lisp:146
 648. `ichiran/dict:query-parents-kanji`  — fn, dict.lisp:400  *[extracted: tatoeba]*
 649. `ichiran/dict:best-kana-conj`  — fn, dict.lisp:428  *[extracted: tatoeba]*
 650. `ichiran/dict:true-kana`  — gf, dict.lisp:0
 651. `ichiran/dict:true-kanji`  — gf, dict.lisp:0
 652. `ichiran/kanji:reading`  — dao, kanji.lisp:42  *[ported]*
 653. `ichiran/kanji:get-reading-alternatives`  — fn, kanji.lisp:216
 654. `ichiran/kanji:*reading-cache*`  — global, kanji.lisp:199
 655. `ichiran/kanji:kanji`  — dao, kanji.lisp:10  *[ported]*
 656. `ichiran/kanji:get-readings-cache`  — fn, kanji.lisp:199
 657. `ichiran/kanji:get-normal-readings`  — fn, kanji.lisp:231
 658. `ichiran/kanji:make-rmap`  — fn, kanji.lisp:273
 659. `ichiran/kanji:match-readings*`  — fn, kanji.lisp:241
 660. `ichiran/kanji:match-readings`  — fn, kanji.lisp:292
 661. `ichiran/dict:check-easy-hints`  — fn, dict-split.lisp:950
 662. `ichiran/dict:conj-prop-json`  — fn, dict.lisp:283
 663. `ichiran/dict:find-words-seqs`  — fn, dict.lisp:520
 664. `ichiran/dict:get-original-text-once`  — fn, dict.lisp:369
 665. `ichiran/dict:match-kana-kanji`  — fn, dict.lisp:1507
 666. `ichiran/dict:match-sense-restrictions`  — fn, dict.lisp:1515
 667. `ichiran/dict:split-pos`  — fn, dict.lisp:1535
 668. `ichiran/dict:get-senses-json`  — fn, dict.lisp:1537
 669. **CYCLE (2 symbols — port together)**
        - `ichiran/dict:conj-info-json`  — fn, dict.lisp:1698
        - `ichiran/dict:conj-info-json*`  — fn, dict.lisp:1665
 670. `ichiran/dict:conjugate-word`  — fn, dict-load.lisp:294
 671. `ichiran/dict:get-digit`  — fn, dict-counters.lisp:94  *[ported]*  *[extracted: tatoeba]*  *[audited 193/193]*
 672. `ichiran/numbers:*digit-to-kana*`  — global, numbers.lisp:25  *[ported]*
 673. `ichiran/numbers:*power-to-kana*`  — global, numbers.lisp:28  *[ported]*
 674. `ichiran/dict:counter-join`  — gf, dict-counters.lisp:0
 675. `ichiran/dict:csv-hash`  — macro, dict-load.lisp:201
 676. `ichiran/dict:defsuffix`  — macro, dict-grammar.lisp:342
 677. `ichiran/dict:def-abbr-suffix`  — macro, dict-grammar.lisp:557
 678. `ichiran/dict:defsplit`  — macro, dict-split.lisp:5
 679. `ichiran/dict:def-simple-split`  — macro, dict-split.lisp:11
 680. `ichiran/dict:def-de-split`  — macro, dict-split.lisp:81
 681. `ichiran/dict:def-do-split`  — macro, dict-split.lisp:181
 682. `ichiran/dict:defhint`  — macro, dict-split.lisp:892
 683. `ichiran/dict:insert-hints`  — fn, dict-split.lisp:875
 684. `ichiran/dict:translate-hint-position`  — fn, dict-split.lisp:930
 685. `ichiran/dict:translate-hints`  — fn, dict-split.lisp:942
 686. `ichiran/dict:def-easy-hint`  — macro, dict-split.lisp:955
 687. `ichiran/dict:defpenalty`  — macro, dict-grammar.lisp:981
 688. `ichiran/dict:def-generic-penalty`  — macro, dict-grammar.lisp:984
 689. `ichiran/dict:defsynergy`  — macro, dict-grammar.lisp:738
 690. `ichiran/dict:def-generic-synergy`  — macro, dict-grammar.lisp:739
 691. `ichiran/dict:def-reader-for-json`  — macro, dict.lisp:1289
 692. `ichiran/dict:defsegfilter`  — macro, dict-grammar.lisp:1043
 693. `ichiran/dict:def-segfilter-must-follow`  — macro, dict-grammar.lisp:1049
 694. `ichiran/dict:def-shi-split`  — macro, dict-split.lisp:191
 695. `ichiran/dict:def-simple-hint`  — macro, dict-split.lisp:901
 696. `ichiran/dict:def-simple-suffix`  — macro, dict-grammar.lisp:345
 697. `ichiran/dict:def-special-counter`  — macro, dict-counters.lisp:361
 698. `ichiran/dict:def-toori-split`  — macro, dict-split.lisp:143
 699. `ichiran/dict:delete-duplicate-props`  — fn, dict.lisp:295
 700. `ichiran/dict:drop-extras`  — fn, dict-load.lisp:194
 701. `ichiran/dict:entry-digest`  — fn, dict.lisp:64
 702. `ichiran/dict:entry-info-long`  — fn, dict.lisp:1601
 703. `ichiran/dict:exists-reading`  — fn, dict.lisp:1847
 704. `ichiran/dict:filter-is-pos`  — macro, dict-grammar.lisp:772
 705. `ichiran/dict:find-word-kana-pattern`  — fn, dict.lisp:1877
 706. `ichiran/dict:find-kanji-for-pattern`  — fn, dict.lisp:1882
 707. `ichiran/dict:find-word-info`  — fn, dict.lisp:1850
 708. `ichiran/dict:word-info-reading`  — fn, dict.lisp:1445
 709. `ichiran/dict:word-info-gloss-json`  — fn, dict.lisp:1784
 710. `ichiran/dict:find-word-info-json`  — fn, dict.lisp:1872
 711. `ichiran/dict:fix-entities`  — fn, dict-load.lisp:159
 712. `ichiran/dict:get-hint`  — fn, dict-split.lisp:968
 713. `ichiran/dict:get-kanji-kana-old`  — fn, dict.lisp:115  *[extracted: tatoeba]*
 714. `ichiran/dict:get-kanji-words`  — fn, dict.lisp:1836
 715. `ichiran/dict:init-tables`  — fn, dict-load.lisp:3
 716. `ichiran/dict:length-multiplier`  — fn, dict.lisp:681
 717. `ichiran/dict:load-best-readings`  — fn, dict-load.lisp:530
 718. `ichiran/dict:load-conjugations`  — fn, dict-load.lisp:445
 719. `ichiran/dict:recalc-entry-stats-all`  — fn, dict.lisp:59
 720. `ichiran/dict:load-extras`  — fn, dict-load.lisp:183
 721. `ichiran/dict:load-jmdict`  — fn, dict-load.lisp:168
 722. `ichiran/dict:recalc-entry-stats`  — fn, dict.lisp:53
 723. `ichiran/dict:word-info-json`  — fn, dict.lisp:1262
 724. `ichiran/dict:simple-word-info`  — fn, dict.lisp:1282
 725. `ichiran/dict:split-kigatsuku`  — fn, dict-split.lisp:298
 726. `ichiran/dict:substring-index`  — fn, dict.lisp:1132
 727. `ichiran/dict:suffix-sou-base`  — macro, dict-grammar.lisp:445
 728. `ichiran/dict:word-info-from-text`  — fn, dict.lisp:1382
 729. `ichiran/dict:word-info-rec-find`  — fn, dict.lisp:1409
 730. `ichiran/dict:word-readings`  — fn, dict.lisp:536
 731. `ichiran/kanji:*kanjidic-path*`  — global, settings.lisp:16
 732. `ichiran/kanji:calculate-perc`  — fn, kanji.lisp:349
 733. `ichiran/kanji:first-node-text`  — fn, kanji.lisp:106
 734. `ichiran/kanji:get-original-reading`  — fn, kanji.lisp:308
 735. `ichiran/kanji:get-reading-stats`  — fn, kanji.lisp:399
 736. `ichiran/kanji:get-readings`  — fn, kanji.lisp:211
 737. `ichiran/kanji:meaning`  — dao, kanji.lisp:83  *[ported]*
 738. `ichiran/kanji:okurigana`  — dao, kanji.lisp:67  *[ported]*
 739. `ichiran/kanji:init-tables`  — fn, kanji.lisp:98
 740. `ichiran:*hepburn-basic*`  — global, romanize.lisp:144
 741. `ichiran/kanji:reading-info-json`  — fn, kanji.lisp:354
 742. `ichiran/kanji:to-json`  — gf, kanji.lisp:0
 743. `ichiran/kanji:kanji-info-json`  — fn, kanji.lisp:392
 744. `ichiran/kanji:kanji-reading-json`  — fn, kanji.lisp:410
 745. `ichiran/kanji:kanji-word-stats`  — fn, kanji.lisp:316
 746. `ichiran/kanji:load-readings`  — fn, kanji.lisp:114
 747. `ichiran/kanji:load-kanji`  — fn, kanji.lisp:152
 748. `ichiran/kanji:load-kanji-stats`  — fn, kanji.lisp:332
 749. `ichiran/kanji:load-kanjidic`  — fn, kanji.lisp:185
 750. `ichiran/kanji:process-match-json`  — fn, kanji.lisp:428
 751. `ichiran/kanji:match-readings-json`  — fn, kanji.lisp:452
 752. `ichiran/kanji:query-kanji-json`  — macro, kanji.lisp:458
 753. `ichiran/numbers:*char-number-class*`  — global, numbers.lisp:9  *[ported]*
 754. `ichiran/numbers:*char-number-class-hash*`  — global, numbers.lisp:18  *[ported]*
 755. `ichiran/numbers:*digit-kanji-legal*`  — global, numbers.lisp:5  *[ported]*
 756. `ichiran/numbers:num-sandhi`  — gf, numbers.lisp:0  *[ported]*
 757. `ichiran/numbers:group-to-kana`  — fn, numbers.lisp:117  *[ported]*
 758. `ichiran/numbers:number-to-kana`  — fn, numbers.lisp:125  *[ported]*
 759. `ichiran/numbers:parse-number*`  — fn, numbers.lisp:57  *[ported]*
 760. `ichiran/numbers:parse-number`  — fn, numbers.lisp:77  *[ported]*
 761. `ichiran:modified-hepburn`  — class, romanize.lisp:162
 762. `ichiran:*hepburn-modified*`  — global, romanize.lisp:168
 763. `ichiran:*hepburn-passport*`  — global, romanize.lisp:149
 764. `ichiran:*kunrei-siki*`  — global, romanize.lisp:201
 765. `ichiran:rmap-item`  — struct, deromanize.lisp:5  *[ported]*
 766. `ichiran:*romaji-kana*`  — global, deromanize.lisp:0
 767. `ichiran:has-successors`  — fn, deromanize.lisp:11
 768. `ichiran:*romaji-kana-next*`  — global, deromanize.lisp:21
 769. `ichiran:kana-representation`  — struct, deromanize.lisp:23  *[ported]*
 770. `ichiran:possible-long-vowel-p`  — fn, deromanize.lisp:30
 771. `ichiran:apply-rmap-item`  — fn, deromanize.lisp:35
 772. `ichiran:join-branches`  — fn, deromanize.lisp:54
 773. `ichiran:kr-concat`  — fn, deromanize.lisp:23
 774. `ichiran:load-romaji-kana`  — fn, deromanize.lisp:5
 775. `ichiran:get-romaji-kana`  — fn, deromanize.lisp:5
 776. `ichiran:romaji-next`  — fn, deromanize.lisp:46
 777. `ichiran:branches-next`  — fn, deromanize.lisp:69
 778. `ichiran:romaji-kana`  — fn, deromanize.lisp:84
 779. `ichiran:romaji-suggest`  — fn, deromanize.lisp:95
