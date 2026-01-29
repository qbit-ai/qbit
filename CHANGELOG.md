# Changelog

## [0.2.15](https://github.com/qbit-ai/qbit/compare/v0.2.14...v0.2.15) (2026-01-29)


### Features

* add image attachment functionality to AgentMessage and UnifiedInput ([#214](https://github.com/qbit-ai/qbit/issues/214)) ([a2b15ae](https://github.com/qbit-ai/qbit/commit/a2b15ae8dadd35e10887b56fbfb981a2db5a1a39))
* **home:** add Home View with projects and worktree management ([#215](https://github.com/qbit-ai/qbit/issues/215)) ([13b6b0f](https://github.com/qbit-ai/qbit/commit/13b6b0fa621b16a73d55182b205e714ecdef9810))


### Bug Fixes

* **e2e:** update tests for Home tab and add missing mock handlers ([#219](https://github.com/qbit-ai/qbit/issues/219)) ([e7f69a0](https://github.com/qbit-ai/qbit/commit/e7f69a0036f3f979fdabc9494b52b854bcee2091))
* **ui:** notification z-index and hide run_command from timeline ([#218](https://github.com/qbit-ai/qbit/issues/218)) ([6f023ba](https://github.com/qbit-ai/qbit/commit/6f023ba70e9de59e79240fea1f53b47cd72f7beb))
* **vision:** add vertex_gemini to vision-capable providers ([#217](https://github.com/qbit-ai/qbit/issues/217)) ([fe0cefc](https://github.com/qbit-ai/qbit/commit/fe0cefc9b25af76f92ae4004daa406a301e01309))

## [0.2.14](https://github.com/qbit-ai/qbit/compare/v0.2.13...v0.2.14) (2026-01-28)


### Features

* **history:** add persistent command and prompt history system ([#210](https://github.com/qbit-ai/qbit/issues/210)) ([860a6a6](https://github.com/qbit-ai/qbit/commit/860a6a620ba529f1d717e40383efbaa50753d91b))
* **llm-providers:** add Gemini on Vertex AI provider ([#206](https://github.com/qbit-ai/qbit/issues/206)) ([f0c4c8d](https://github.com/qbit-ai/qbit/commit/f0c4c8d20d0b024032211f6d081f651fbf197b18))
* **rig-openai-responses:** add image support for user messages ([#207](https://github.com/qbit-ai/qbit/issues/207)) ([bff0596](https://github.com/qbit-ai/qbit/commit/bff0596dff04cf62084fa03647215ab80133c5c4))
* **shell:** add streaming output for run_command tool ([#211](https://github.com/qbit-ai/qbit/issues/211)) ([36d9003](https://github.com/qbit-ai/qbit/commit/36d90033e498924bc6467e202c698b5c5d08271f))


### Bug Fixes

* **biome:** resolve worktree config conflicts ([#213](https://github.com/qbit-ai/qbit/issues/213)) ([b83a2b2](https://github.com/qbit-ai/qbit/commit/b83a2b2a57e642c669b3ef91c1a03c6e774c3eb3))


### Refactoring

* **file-editor:** use single shared instance across all tabs ([#208](https://github.com/qbit-ai/qbit/issues/208)) ([5c7e935](https://github.com/qbit-ai/qbit/commit/5c7e935ee3a8bb144276dfb92f362855f4ac3169))
* **workflows:** simplify environment variable usage in evals and update-homebrew workflows ([ce58bb6](https://github.com/qbit-ai/qbit/commit/ce58bb6f67c8200d4315513ac53f841ae13a36cc))

## [0.2.13](https://github.com/qbit-ai/qbit/compare/v0.2.12...v0.2.13) (2026-01-26)


### Refactoring

* **input:** remove placeholder text and use data attributes for E2E tests ([#202](https://github.com/qbit-ai/qbit/issues/202)) ([92a0206](https://github.com/qbit-ai/qbit/commit/92a02062900f1ee10109608381787760efb4a7b6))

## [0.2.12](https://github.com/qbit-ai/qbit/compare/v0.2.11...v0.2.12) (2026-01-26)


### Refactoring

* provider-model config consolidation (Phases 1-4) ([#199](https://github.com/qbit-ai/qbit/issues/199)) ([5b2e570](https://github.com/qbit-ai/qbit/commit/5b2e5706bd55500b6ab85808e52487d05c392fc3))

## [0.2.11](https://github.com/qbit-ai/qbit/compare/v0.2.10...v0.2.11) (2026-01-26)


### Features

* **path-completion:** enhance tab completion with fuzzy matching and file type icons ([#198](https://github.com/qbit-ai/qbit/issues/198)) ([4e8d069](https://github.com/qbit-ai/qbit/commit/4e8d0693b242650d7dcda3dd364433b7563219e0))
* **timeline:** implement Phase 2 performance improvements and reliability fixes ([#194](https://github.com/qbit-ai/qbit/issues/194)) ([1e4452c](https://github.com/qbit-ai/qbit/commit/1e4452c917c11ca68dcc2cb68af25e43aac6b69b))


### Refactoring

* **store:** implement single source of truth for timeline data ([#197](https://github.com/qbit-ai/qbit/issues/197)) ([dddc37c](https://github.com/qbit-ai/qbit/commit/dddc37ca848d7786aff36f56e562a6c2a3560ba6))

## [0.2.10](https://github.com/qbit-ai/qbit/compare/v0.2.9...v0.2.10) (2026-01-25)


### Features

* **ai:** implement EventCoordinator for deadlock-free event management ([#185](https://github.com/qbit-ai/qbit/issues/185)) ([cbe18c7](https://github.com/qbit-ai/qbit/commit/cbe18c7a3a5cddeab408ac2a4d9a73ba77f66eb6))
* **ai:** replace Z.AI providers with unified rig-zai-sdk ([#191](https://github.com/qbit-ai/qbit/issues/191)) ([7e54a77](https://github.com/qbit-ai/qbit/commit/7e54a77caaa99e5053fb8dc5e64dd895b4784d9f))
* **evals:** SWE-bench Lite integration for agent benchmarking ([#181](https://github.com/qbit-ai/qbit/issues/181)) ([0b544ba](https://github.com/qbit-ai/qbit/commit/0b544baedc6c3774d356b1d12f1f859d76a73bf4))
* **openai:** add reasoning effort support and xhigh level for GPT models ([#177](https://github.com/qbit-ai/qbit/issues/177)) ([a495bd3](https://github.com/qbit-ai/qbit/commit/a495bd31ee241813c977ddac0a483f0a6d42aecb))
* **swebench:** integrate official SWE-bench harness for test evaluation ([22d180c](https://github.com/qbit-ai/qbit/commit/22d180c2ff20cb07dea044de1fd0127374190e6f))


### Bug Fixes

* **ai:** improve event reliability and prevent directory change deadlock ([#184](https://github.com/qbit-ai/qbit/issues/184)) ([0cc9bec](https://github.com/qbit-ai/qbit/commit/0cc9becdfe9782a44002606e3c96ea40b5e96d81))
* **ai:** OpenAI temperature regression, UTF-8 panic, and rig-core upgrade ([#187](https://github.com/qbit-ai/qbit/issues/187)) ([928de26](https://github.com/qbit-ai/qbit/commit/928de26e92954b00ccfcf26bcea4cf97ac21ec32))
* **ai:** resolve deadlock in release builds when switching models ([c270f5c](https://github.com/qbit-ai/qbit/commit/c270f5c57dee9cb580f6447ca5c332312e18530d))
* **ai:** resolve deadlock in release builds when switching models ([#179](https://github.com/qbit-ai/qbit/issues/179)) ([fba5466](https://github.com/qbit-ai/qbit/commit/fba5466e18fce03b9cfed7067619d2e8c5905135))
* **openai:** fix reasoning display and history for Responses API ([#180](https://github.com/qbit-ai/qbit/issues/180)) ([66165a0](https://github.com/qbit-ai/qbit/commit/66165a0abc67621671b21e7ff866a8d403e9abd1))
* remount race conditions ([#190](https://github.com/qbit-ai/qbit/issues/190)) ([a957551](https://github.com/qbit-ai/qbit/commit/a95755185fffb2bb484191fadc6cbd2f25b2584d))


### Refactoring

* **settings:** remove obsolete Z.AI and Z.AI (Anthropic) providers in favor of ZaiSdk ([#192](https://github.com/qbit-ai/qbit/issues/192)) ([8f2c059](https://github.com/qbit-ai/qbit/commit/8f2c05950a15a3fa8cc263fca7653951a9385611))

## [0.2.9](https://github.com/qbit-ai/qbit/compare/v0.2.8...v0.2.9) (2026-01-18)


### Features

* **events:** add reasoning field to completion events and reduce log noise ([#168](https://github.com/qbit-ai/qbit/issues/168)) ([90e71d1](https://github.com/qbit-ai/qbit/commit/90e71d1fec3cb684d5138f786fd64cbba4d3703a))
* **skills:** add Agent Skills support with agentskills.io spec ([#174](https://github.com/qbit-ai/qbit/issues/174)) ([4edc0f8](https://github.com/qbit-ai/qbit/commit/4edc0f832a686fa9792bafde455051a2fc3c18b1))
* **system-hooks:** add logging, OTel events, and plan completion reminder ([#172](https://github.com/qbit-ai/qbit/issues/172)) ([bfdbb6e](https://github.com/qbit-ai/qbit/commit/bfdbb6ebc2360d2860363ea6aa8cc30afdc7050e))
* **vertex:** enable prompt caching for Anthropic Vertex AI provider ([#171](https://github.com/qbit-ai/qbit/issues/171)) ([6619ad9](https://github.com/qbit-ai/qbit/commit/6619ad9fe1c7b3bc2f5321616c150575dbf6ee26))


### Bug Fixes

* **agentic-loop:** ensure assistant messages are added to history before loop exit ([#170](https://github.com/qbit-ai/qbit/issues/170)) ([1217423](https://github.com/qbit-ai/qbit/commit/12174237e1de9c4830797f9122e1466bab339982))
* e2e tests ([8abd766](https://github.com/qbit-ai/qbit/commit/8abd76606041c2f4cec36856f81e1f8392d2ce7d))
* settings functionality ([b536455](https://github.com/qbit-ai/qbit/commit/b5364555c6f659bd4174c4eda62d33fef6eb8e40))
* settings theme saving ([4dde840](https://github.com/qbit-ai/qbit/commit/4dde8402a8b4768f6f87e772e70f36ed938a2378))
* **settings:** settings bugs ([9d32607](https://github.com/qbit-ai/qbit/commit/9d326070ab84fdac562acc92cbd61ad01067c193))

## [0.2.8](https://github.com/qbit-ai/qbit/compare/v0.2.7...v0.2.8) (2026-01-14)


### Features

* add LLM API request/response logging ([#148](https://github.com/qbit-ai/qbit/issues/148)) ([2a2174a](https://github.com/qbit-ai/qbit/commit/2a2174a2a022573d0f12cbc82787413d47d19b38))
* Add TavilyToolsContributor for system prompt integration ([#142](https://github.com/qbit-ai/qbit/issues/142)) ([18cd66b](https://github.com/qbit-ai/qbit/commit/18cd66bc4c041980cf592c5a0cb0f7bbc6ea0287))
* add transcript recording and context compaction trigger ([#158](https://github.com/qbit-ai/qbit/issues/158)) ([817d867](https://github.com/qbit-ai/qbit/commit/817d867fa13f33f5c8b4c32f5041be35991821d9))
* **ai:** add Z.AI Anthropic-compatible provider ([#149](https://github.com/qbit-ai/qbit/issues/149)) ([a70fbdb](https://github.com/qbit-ai/qbit/commit/a70fbdbf1cb52831ed40cd3a9f02f5782fe68732))
* **context-compaction:** add frontend UI for compaction events ([#165](https://github.com/qbit-ai/qbit/issues/165)) ([d5f0ad5](https://github.com/qbit-ai/qbit/commit/d5f0ad505d09413d586bf1e9bd5636fce0c01089))
* **context-compaction:** implement hard reset mechanism (step 5) ([#163](https://github.com/qbit-ai/qbit/issues/163)) ([0b739e9](https://github.com/qbit-ai/qbit/commit/0b739e9c2b880c561ba7902c2502432880f87810))
* **context-compaction:** implement summarizer agent and compaction trigger ([#159](https://github.com/qbit-ai/qbit/issues/159)) ([51ebaa6](https://github.com/qbit-ai/qbit/commit/51ebaa6888a8916a5e6f31a88c858fad62249636))
* **context-compaction:** implement summarizer input builder ([#161](https://github.com/qbit-ai/qbit/issues/161)) ([ec7b5b9](https://github.com/qbit-ai/qbit/commit/ec7b5b93ea8c5ce0382d793ce8ef94880a3de810))
* **context:** add compaction trigger and multi-model token limits ([#160](https://github.com/qbit-ai/qbit/issues/160)) ([040fd41](https://github.com/qbit-ai/qbit/commit/040fd413d275888fc8a42efd717669955fc444b7))
* **git:** add periodic status polling for status bar badge ([bd5dd23](https://github.com/qbit-ai/qbit/commit/bd5dd235d1639db00fa886f2be3f9efef75e8f59))
* **pty:** initial bash shell integration ([#155](https://github.com/qbit-ai/qbit/issues/155)) ([74ce062](https://github.com/qbit-ai/qbit/commit/74ce0621bbdbd2b88e3d592672330841c9ca4ef4))
* telemetry filtering, API logging, indexer deduplication, and UserMessage fix ([#157](https://github.com/qbit-ai/qbit/issues/157)) ([bc39762](https://github.com/qbit-ai/qbit/commit/bc39762595755ea2c0b3817b53c8f9b3883e7736))
* **vertex-ai:** support application default credentials ([#145](https://github.com/qbit-ai/qbit/issues/145)) ([976b7cf](https://github.com/qbit-ai/qbit/commit/976b7cf749de71b3d1103a26b65743608be45fa2))


### Bug Fixes

* add UTF-8 safe string truncation to prevent panics ([#162](https://github.com/qbit-ai/qbit/issues/162)) ([5546552](https://github.com/qbit-ai/qbit/commit/5546552a8436bd1095e0d731d419a36b16b2c406))
* **ai:** emit error notifications and fix context pruning event ([#141](https://github.com/qbit-ai/qbit/issues/141)) ([bbc5ab7](https://github.com/qbit-ai/qbit/commit/bbc5ab7f71af545b6851b698381f36119023e7d2))
* **context-compaction:** improve trigger timing and timeline display ([#166](https://github.com/qbit-ai/qbit/issues/166)) ([0221220](https://github.com/qbit-ai/qbit/commit/022122096b7493b3fd1af44a21dd7d0bb9b96b91))
* **executor:** ensure sub-agent spans are parented correctly ([#154](https://github.com/qbit-ai/qbit/issues/154)) ([1bb39ad](https://github.com/qbit-ai/qbit/commit/1bb39ad261eeec21f0bedbaf11930b20923d5378))
* **frontend:** add global error handling and fix runtime errors ([#156](https://github.com/qbit-ai/qbit/issues/156)) ([891d95a](https://github.com/qbit-ai/qbit/commit/891d95a0baa023896fda8281bfabc1701b5cef39))
* **git:** show diff for untracked files in GitPanel ([#150](https://github.com/qbit-ai/qbit/issues/150)) ([5e14742](https://github.com/qbit-ai/qbit/commit/5e1474273c5d6154e0612436503a8703b6e8ef1c))
* **pty:** revert parser changes causing terminal visibility issues ([#167](https://github.com/qbit-ai/qbit/issues/167)) ([839d0f8](https://github.com/qbit-ai/qbit/commit/839d0f8a89f2395fa711fe1a41169b002f013877))
* **sub-agents:** include thinking blocks in conversation history ([#151](https://github.com/qbit-ai/qbit/issues/151)) ([5f9ed65](https://github.com/qbit-ai/qbit/commit/5f9ed654d636c37b4298e7def3e2095ac73c4ef3))
* **telemetry:** properly instrument main agentic loop spans ([#139](https://github.com/qbit-ai/qbit/issues/139)) ([1be19b3](https://github.com/qbit-ai/qbit/commit/1be19b3638543cc01b4f21e5ce2aa5daeb0c39e4))
* **ui:** improve AgentMessage layout and copy button positioning ([73dd3f1](https://github.com/qbit-ai/qbit/commit/73dd3f1caba76f2565c1406ac50658317e26ca40))
* update e2e test regex and add auto-approve safeguards ([#152](https://github.com/qbit-ai/qbit/issues/152)) ([ce4452a](https://github.com/qbit-ai/qbit/commit/ce4452aa080bef9c458effaa8f9164d42cec0a8d))


### Refactoring

* **ai:** simplify system prompt structure ([#147](https://github.com/qbit-ai/qbit/issues/147)) ([65cb9ec](https://github.com/qbit-ai/qbit/commit/65cb9ecaabd6fc5c945f899ea2068e4097575d89))
* **context:** remove legacy pruning system ([#164](https://github.com/qbit-ai/qbit/issues/164)) ([d35bd82](https://github.com/qbit-ai/qbit/commit/d35bd821014804a1378bded643b2fd44313737f5))

## [0.2.7](https://github.com/qbit-ai/qbit/compare/v0.2.6...v0.2.7) (2026-01-10)


### Features

* **ci:** add Linux x86_64 build to release workflow ([135b93a](https://github.com/qbit-ai/qbit/commit/135b93a427e7b5cde4d65e1c53dc9fe23d350cd1))
* **ci:** add Linux x86_64 build to release workflow ([1a1b715](https://github.com/qbit-ai/qbit/commit/1a1b71587039a0578a9cddd5439a377b58de19b6))
* **telemetry:** improve Langfuse tracing for sub-agents and LLM spans ([#136](https://github.com/qbit-ai/qbit/issues/136)) ([b1fc749](https://github.com/qbit-ai/qbit/commit/b1fc749c6d1b59b3bba143925c47d905c760d1fe))


### Bug Fixes

* **telemetry:** improve log readability and span nesting ([#138](https://github.com/qbit-ai/qbit/issues/138)) ([149ec5c](https://github.com/qbit-ai/qbit/commit/149ec5cfef7761987af3a9fdb66512ee9a5f466f))

## [0.2.6](https://github.com/qbit-ai/qbit/compare/v0.2.5...v0.2.6) (2026-01-10)


### Features

* **editor:** add vim commands and improve file path detection ([#129](https://github.com/qbit-ai/qbit/issues/129)) ([826721a](https://github.com/qbit-ai/qbit/commit/826721a0e7d42790789247a275c43fc42b47ee93))
* **settings:** render settings as tab instead of modal dialog ([#110](https://github.com/qbit-ai/qbit/issues/110)) ([73aa78c](https://github.com/qbit-ai/qbit/commit/73aa78cc8c876f36cad520a0183e68b2068d63da))
* **sub-agents:** add parent_request_id to correlate sub-agent events ([#125](https://github.com/qbit-ai/qbit/issues/125)) ([1ee748c](https://github.com/qbit-ai/qbit/commit/1ee748cce83001b98eaa36159182c9fff69ab8a9))
* **tools:** add tool group details modal and mixed tool grouping ([#133](https://github.com/qbit-ai/qbit/issues/133)) ([f489979](https://github.com/qbit-ai/qbit/commit/f489979dc8f2cf6b9210539d7f61875cf0a1025f))
* **ui:** add clickable file path links in markdown and terminal ([#128](https://github.com/qbit-ai/qbit/issues/128)) ([a869f44](https://github.com/qbit-ai/qbit/commit/a869f44334f10d7562bf24c745c91f4997c4c02a))


### Bug Fixes

* **agent:** auto-approve mode now bypasses tool policy checks ([#127](https://github.com/qbit-ai/qbit/issues/127)) ([4f2777b](https://github.com/qbit-ai/qbit/commit/4f2777b04a8e3581b6555ce121cd192638aa3018))
* **agent:** resolve tab close and multi-agent initialization issues ([#126](https://github.com/qbit-ai/qbit/issues/126)) ([736518a](https://github.com/qbit-ai/qbit/commit/736518a596ba1624c91e8bf26cb46d998ce2b228))
* **git:** Auto-refresh branch/status after checkout commands ([#124](https://github.com/qbit-ai/qbit/issues/124)) ([65542a4](https://github.com/qbit-ai/qbit/commit/65542a4e4da8eb9cb5c0c90aa7befc730e62db52))
* **session:** fix restore initialization order and add agent_mode persistence ([#130](https://github.com/qbit-ai/qbit/issues/130)) ([0bcb2f7](https://github.com/qbit-ai/qbit/commit/0bcb2f77b4204e6f9f3498612dec5b065e188340))
* **session:** use current default provider when restoring sessions ([#131](https://github.com/qbit-ai/qbit/issues/131)) ([d4d030f](https://github.com/qbit-ai/qbit/commit/d4d030f84df101df76e354ee722d1b34ed73ca15))
* **terminal:** align path completion with standard shell behavior ([#132](https://github.com/qbit-ai/qbit/issues/132)) ([8153206](https://github.com/qbit-ai/qbit/commit/8153206548478826f55d531f63585e873d4cfe81))

## [0.2.5](https://github.com/qbit-ai/qbit/compare/v0.2.4...v0.2.5) (2026-01-08)


### Features

* **ai:** add per-sub-agent model overrides ([#112](https://github.com/qbit-ai/qbit/issues/112)) ([7dd3911](https://github.com/qbit-ai/qbit/commit/7dd3911a80f74722272e395747f92d1305eb4c2e))
* **input:** add argument support for slash commands ([#121](https://github.com/qbit-ai/qbit/issues/121)) ([8677cd1](https://github.com/qbit-ai/qbit/commit/8677cd1db21e54e8c678b42e504e2808020f9808))
* **input:** add multi-modal image input via drag-drop and paste ([#104](https://github.com/qbit-ai/qbit/issues/104)) ([5bff13f](https://github.com/qbit-ai/qbit/commit/5bff13f333e0ba1ffb3a184e4705703ca912fc29))
* **logging:** add persistent file logging and reduce verbosity ([#106](https://github.com/qbit-ai/qbit/issues/106)) ([7b727b4](https://github.com/qbit-ai/qbit/commit/7b727b44325bda76ba779a9cde4f69f502106172))
* **settings:** add per-project AI settings persistence ([#115](https://github.com/qbit-ai/qbit/issues/115)) ([fe4a32a](https://github.com/qbit-ai/qbit/commit/fe4a32ac6f4d75ee86c3af28e354710ae6f0e931))
* **terminal:** replace ANSI text output with embedded xterm.js terminals ([#111](https://github.com/qbit-ai/qbit/issues/111)) ([3f1911d](https://github.com/qbit-ai/qbit/commit/3f1911d1fa8a98bd2f29b3253ec90532bb76430e))
* **ui:** add copy buttons to user messages and command blocks ([1067f2b](https://github.com/qbit-ai/qbit/commit/1067f2b160284323879944d574070577324cae14))
* **ui:** add copy buttons to user messages and command blocks ([75159ec](https://github.com/qbit-ai/qbit/commit/75159eca5d13d2c27c93fba6001434026841fd63))
* **ui:** add details modal for sub-agent cards in timeline ([#116](https://github.com/qbit-ai/qbit/issues/116)) ([70983b5](https://github.com/qbit-ai/qbit/commit/70983b5aef26b5448f0f68d4c259589cdcbfc702))


### Bug Fixes

* close tab button not working with active agent/running command ([#118](https://github.com/qbit-ai/qbit/issues/118)) ([378b7c6](https://github.com/qbit-ai/qbit/commit/378b7c61ce8410a3c997676cde71073b1baf0d47))
* **e2e:** use globally exposed mocks for timeline scroll tests ([#107](https://github.com/qbit-ai/qbit/issues/107)) ([9086695](https://github.com/qbit-ai/qbit/commit/9086695dc20fc2a651fdd446cce8a084fac3066f))
* **input:** improve arrow key history navigation and command block handling ([#114](https://github.com/qbit-ai/qbit/issues/114)) ([623fc83](https://github.com/qbit-ai/qbit/commit/623fc8386acc6a6b62b53967c4faadc1dc9e2cde))
* **session:** sync session workspace path when cwd changes ([#122](https://github.com/qbit-ai/qbit/issues/122)) ([7053340](https://github.com/qbit-ai/qbit/commit/70533406f00ccf3ae92d76c3f82748c21f95d63f))
* **ui:** apply agent mode to backend when loading project settings ([#119](https://github.com/qbit-ai/qbit/issues/119)) ([4d2b516](https://github.com/qbit-ai/qbit/commit/4d2b5166299c775aaf904a17e2210009de312839))
* **ui:** remove git loading spinner and improve streaming auto-scroll ([#117](https://github.com/qbit-ai/qbit/issues/117)) ([f385d64](https://github.com/qbit-ai/qbit/commit/f385d64656ea0d065d2b0bdee82c40a04fa4abef))


### Refactoring

* **qbit:** use if let instead of match for single variant ([d5d3470](https://github.com/qbit-ai/qbit/commit/d5d347099b34588d621862b41e979a85af5b24af))
* **sub-agents:** use natural language output for analyzer and explorer ([#120](https://github.com/qbit-ai/qbit/issues/120)) ([3751bf0](https://github.com/qbit-ai/qbit/commit/3751bf09680c7e68a3cba8d486cb7854b18b9131))
* **window:** move window state persistence from frontend to Rust backend ([e772fc0](https://github.com/qbit-ai/qbit/commit/e772fc0157a8bb484886dbd0d948a0fad4472e6f))

## [0.2.4](https://github.com/qbit-ai/qbit/compare/v0.2.3...v0.2.4) (2026-01-06)


### Features

* **ai:** add dynamic prompt composition system ([6215cdc](https://github.com/qbit-ai/qbit/commit/6215cdccbf0ac49e9e8a424214372633b5fd04fe))
* **ai:** add multi-modal image attachment support ([#101](https://github.com/qbit-ai/qbit/issues/101)) ([bd1b836](https://github.com/qbit-ai/qbit/commit/bd1b83681074943c22fe9d33d3705296b5f7c205))
* **ai:** add OpenAI native web search integration ([b6525d5](https://github.com/qbit-ai/qbit/commit/b6525d56f3e6f63033ba1dd6ba8f66931d50e125))
* **capabilities:** enhance Z.AI support with preserved thinking mode and reasoning continuity ([3c823fe](https://github.com/qbit-ai/qbit/commit/3c823fe6d6c51fe77af480fc68ef9c5d28dae5d5))
* **evals:** add metric pass threshold logic for providers ([0433627](https://github.com/qbit-ai/qbit/commit/0433627d8634b39120d2066a399bca44b971b46e))
* **evals:** add OpenAI model scenarios and connectivity test framework ([4253ec7](https://github.com/qbit-ai/qbit/commit/4253ec7e10f2fb623e41b206ccab1ae1537fd9da))
* **evals:** add OpenAI provider and upgrade rig-core to 0.27.0 ([#82](https://github.com/qbit-ai/qbit/issues/82)) ([6adee68](https://github.com/qbit-ai/qbit/commit/6adee68c58d8da93c8f0285c6c5e450a991e2078))
* **evals:** add Z.AI GLM-4.7 provider support ([#75](https://github.com/qbit-ai/qbit/issues/75)) ([cb8c722](https://github.com/qbit-ai/qbit/commit/cb8c72210a0cfce7136be757ab6c3352081818a8))
* **evals:** align eval system prompts with production agent ([#95](https://github.com/qbit-ai/qbit/issues/95)) ([5fbc8c5](https://github.com/qbit-ai/qbit/commit/5fbc8c5c96bc5b520848b122623e59dfb7829425))
* **sub-agents:** add sub-agent support with timeline integration and E2E tests ([0f3a768](https://github.com/qbit-ai/qbit/commit/0f3a768eee38ddf015ed71cd3e048030508b5d0c))
* **terminal:** add portal-based rendering for Terminal state persistence ([89bc8bf](https://github.com/qbit-ai/qbit/commit/89bc8bff71803b2895b7aba6e5345b1c8b8be32d))
* **terminal:** add React portal architecture for Terminal persistence ([bdd0d5d](https://github.com/qbit-ai/qbit/commit/bdd0d5dc434d9a41194893e6aa40ef04f6c8fcfa))
* **terminal:** add TerminalInstanceManager for cross-remount persistence ([2b300ce](https://github.com/qbit-ai/qbit/commit/2b300ce3a6c981cbbab454d33c09260061c6ac74))
* **terminal:** integrate portal system and preserve tab state ([de3e40e](https://github.com/qbit-ai/qbit/commit/de3e40e58bc97e3a6ad1292925d2bd3abb817992))
* **tools:** add ast-grep tools for structural code search and replace ([#94](https://github.com/qbit-ai/qbit/issues/94)) ([ab15841](https://github.com/qbit-ai/qbit/commit/ab158416578852015a8d7f41cb83707edc58a70b))
* **ui:** add 3-level nested model selector with temperature support ([b729dbf](https://github.com/qbit-ai/qbit/commit/b729dbf33db1c70a2d35775cb029a2974f120ae8))
* **ui:** add comprehensive OpenAI model support ([#83](https://github.com/qbit-ai/qbit/issues/83)) ([281135a](https://github.com/qbit-ai/qbit/commit/281135a3c01f29cf4e79bb9b0c7e7bfc25ca6939))
* **web-tools:** add native web search and web fetch support for Claude ([35044cf](https://github.com/qbit-ai/qbit/commit/35044cfed47984ac08520d8e45a117fac7f3cfce))
* **workflows:** implement new workflows with structured schemas ([f85aafb](https://github.com/qbit-ai/qbit/commit/f85aafbb598f2ab677bf95c9f00dee203f41e7f7))


### Bug Fixes

* **ai:** preserve OpenAI Responses API reasoning IDs across turns ([#92](https://github.com/qbit-ai/qbit/issues/92)) ([1793e66](https://github.com/qbit-ai/qbit/commit/1793e66fe02d64cd73713c595938039fda35f15f))
* **ci:** enable ad-hoc code signing for macOS builds ([86003bf](https://github.com/qbit-ai/qbit/commit/86003bfdca87d475098e8114505d39d31e5c9d28))
* **ci:** enable ad-hoc code signing for macOS builds ([dd63a26](https://github.com/qbit-ai/qbit/commit/dd63a2611488584270f82657883a1c3d7a1ddb72))
* **ci:** only run release build when release is created ([1ecb190](https://github.com/qbit-ai/qbit/commit/1ecb1901b5c1fca4c4761518344d8111863153b0))
* **ci:** only run release build when release is created ([402ba93](https://github.com/qbit-ai/qbit/commit/402ba93e587263219504c5607cbc54b2635f9ffe))
* **e2e:** exclude xterm helper textarea from selectors ([9572fd9](https://github.com/qbit-ai/qbit/commit/9572fd94d3bce3c35e14559d02430a8f0462c6b5))
* **e2e:** replace non-null assertions with proper null checks ([5610bb6](https://github.com/qbit-ai/qbit/commit/5610bb6fd03fd26e137df6c05a773e6dc1f35f8b))
* **e2e:** update OpenAI model tests for nested dropdown menus ([e79454b](https://github.com/qbit-ai/qbit/commit/e79454b682ca53a538ea408a1e7c1ce735f82d5e))
* **evals:** add ast-grep tools to eval system prompt and fix LLM score parsing ([#99](https://github.com/qbit-ai/qbit/issues/99)) ([54625a9](https://github.com/qbit-ai/qbit/commit/54625a9394afcb9e95732dea10ef56d1efd965ad))
* **evals:** improve eval reliability and build performance ([#85](https://github.com/qbit-ai/qbit/issues/85)) ([718fd2f](https://github.com/qbit-ai/qbit/commit/718fd2fa09e6f57a0da80d60f862944382a04567))
* **evals:** improve LLM judge reliability and prompt composition tests ([#80](https://github.com/qbit-ai/qbit/issues/80)) ([4322ca9](https://github.com/qbit-ai/qbit/commit/4322ca9793dc3c7788d41b66be8338f9ff214d34))
* file editor dirty/clean indicator now correctly reflects undo state ([#102](https://github.com/qbit-ai/qbit/issues/102)) ([e8f84a3](https://github.com/qbit-ai/qbit/commit/e8f84a32145535b4fa8853962d7d3af2053f567c))
* **keybinds:** separate Ctrl+D close from Cmd+D split on macOS ([54405ba](https://github.com/qbit-ai/qbit/commit/54405bae74cde07099c249ef4ee5943ccc046254))
* **pty:** fall back to home directory when cwd is root ([683465d](https://github.com/qbit-ai/qbit/commit/683465de16e79ae76002825377b48309377baef6))
* **pty:** fall back to home directory when cwd is root ([234d94c](https://github.com/qbit-ai/qbit/commit/234d94c14df6be20b1bf7f13c9d4a8097da0c0e2))
* **shell:** load PATH from shell rc files in run_command tool ([407c686](https://github.com/qbit-ai/qbit/commit/407c686d22ae66198bb64d1d2ea4eed56febc065))
* **shell:** load PATH from shell rc files in run_command tool ([#96](https://github.com/qbit-ai/qbit/issues/96)) ([f401404](https://github.com/qbit-ai/qbit/commit/f40140460ca565b0a4bc65a93a69cb53386b0b54))
* **terminal:** improve initialization and fullterm mode transitions ([e6da594](https://github.com/qbit-ai/qbit/commit/e6da5949b6f111fc794e67ce152d485a286add23))
* **terminal:** improve resize debouncing and pane focus handling ([4e8d48c](https://github.com/qbit-ai/qbit/commit/4e8d48c7f22d25e411e8ca24f35cc537ce349468))


### Refactoring

* **ai:** consolidate agentic loop implementations ([#87](https://github.com/qbit-ai/qbit/issues/87)) ([c1c20eb](https://github.com/qbit-ai/qbit/commit/c1c20eb7d3e82c4e96cd2ed68d31e0ea919abc3c))
* **ai:** redesign system prompts with structured XML format ([#89](https://github.com/qbit-ai/qbit/issues/89)) ([432ee8e](https://github.com/qbit-ai/qbit/commit/432ee8ebf2e3c3f7c96a0c09685681b393ca54d8))
* **build:** improve test and check scripts with silent outputs and clearer messaging ([4f329de](https://github.com/qbit-ai/qbit/commit/4f329de994e263d3d0d89dcf15f66bd1abf0cf45))
* **build:** improve test and check scripts with silent outputs and clearer messaging ([8e1b5ba](https://github.com/qbit-ai/qbit/commit/8e1b5ba0afcaec08bdc84a4e89ac3f72b9f0d2ef))
* **evals:** use &Path instead of &PathBuf in LLM judge helpers ([5bc6801](https://github.com/qbit-ai/qbit/commit/5bc68012366d23aa6515b73891d4fdc22a803f9a))

## [0.2.3](https://github.com/qbit-ai/qbit/compare/v0.2.2...v0.2.3) (2025-12-31)


### Bug Fixes

* **ci:** build qbit-cli sidecar for release bundling ([#72](https://github.com/qbit-ai/qbit/issues/72)) ([b6cc102](https://github.com/qbit-ai/qbit/commit/b6cc1025409becb394e32bd099d397d5eaa3555f))

## [0.2.2](https://github.com/qbit-ai/qbit/compare/v0.2.1...v0.2.2) (2025-12-31)


### Bug Fixes

* **ci:** configure Tauri action project path for release builds ([#70](https://github.com/qbit-ai/qbit/issues/70)) ([d63c464](https://github.com/qbit-ai/qbit/commit/d63c464dc913cf07d53754cea3ad3f69373d70be))

## [0.2.1](https://github.com/qbit-ai/qbit/compare/v0.2.0...v0.2.1) (2025-12-30)


### Features

* **ai:** add OpenAI Responses API support and standardize temperature ([#67](https://github.com/qbit-ai/qbit/issues/67)) ([debae67](https://github.com/qbit-ai/qbit/commit/debae67f41bd41ee52d445c500e515b171e51815))
* **ui:** add multi-pane support for split terminal layouts ([#65](https://github.com/qbit-ai/qbit/issues/65)) ([0d3d306](https://github.com/qbit-ai/qbit/commit/0d3d306577bcb77935d3bcfaa6986a8055856225))


### Bug Fixes

* **build:** specify mainBinaryName to fix macOS release bundling ([#68](https://github.com/qbit-ai/qbit/issues/68)) ([c43ccd0](https://github.com/qbit-ai/qbit/commit/c43ccd05e29734ae3f8cb65ea32a741af5730877))

## [0.2.0](https://github.com/qbit-ai/qbit/compare/v0.1.0...v0.2.0) (2025-12-29)


### ⚠ BREAKING CHANGES

* **sidecar:** Sidecar API completely rewritten.

### Features

* add conversation-level token usage tracking ([#49](https://github.com/qbit-ai/qbit/issues/49)) ([ac21420](https://github.com/qbit-ai/qbit/commit/ac214209d6e846540b835485f579fba50deab170))
* add event mocking support to Tauri IPC mocks ([6cccd44](https://github.com/qbit-ai/qbit/commit/6cccd44e21e9718acf058b2f37208eae13816b31))
* add MockDevTools panel for browser-mode development ([926edc3](https://github.com/qbit-ai/qbit/commit/926edc31718bb09e2313a6b2dbb4a2c970ea6a10))
* add OpenRouter model support ([397d79b](https://github.com/qbit-ai/qbit/commit/397d79b27bf326086326c4da158cbe1e445ddca4))
* add path completion commands and React hook for use in Tauri terminals ([3715bb0](https://github.com/qbit-ai/qbit/commit/3715bb0d2a59b4654e2e71081ed2d07b22a8e29a))
* add preset scenarios to MockDevTools ([d76370c](https://github.com/qbit-ai/qbit/commit/d76370c189ad31e24a3e8094f8c2e0e20c8241a8))
* add Tauri IPC mock adapter for browser-only development ([74b3fe3](https://github.com/qbit-ai/qbit/commit/74b3fe3cc5c683b798075e30d0feb101410f8b24))
* add theme settings to settings ([6b0bc95](https://github.com/qbit-ai/qbit/commit/6b0bc95a3194d08eb5e67ae77bad0486086f419e))
* Add theme system with background image support ([26367bf](https://github.com/qbit-ai/qbit/commit/26367bf559b358398c904451c95fb5cf4f5cb629))
* **ai:** add agent mode support for flexible tool approval behavior ([7420204](https://github.com/qbit-ai/qbit/commit/742020469dce253708a0c147ed9aae4cbf2b6929))
* **ai:** add dynamic memory file lookup from settings ([e1ffaec](https://github.com/qbit-ai/qbit/commit/e1ffaec2434a9b8a825d932ca1a6c2e9c4668f4e))
* **ai:** add extended thinking mode and UI for reasoning content ([d95dd0d](https://github.com/qbit-ai/qbit/commit/d95dd0dfddbad2f96ce7230ab941943d9eca6de6))
* **ai:** add multi-provider support for Anthropic, Ollama, Gemini, Groq, and xAI ([708b804](https://github.com/qbit-ai/qbit/commit/708b804499a43dd32ec20f8168255a872d57e0f2))
* **ai:** add multi-provider support for Anthropic, Ollama, Gemini, Groq, and xAI ([2f86e9c](https://github.com/qbit-ai/qbit/commit/2f86e9cfc536687fb52c2969621923ce7db2ffe7))
* **ai:** add OpenAI provider support ([9fba524](https://github.com/qbit-ai/qbit/commit/9fba524a231574ea1a6db27843ebac7d2f7f87b3))
* **ai:** add OpenAI provider support to Rust backend ([9591ffc](https://github.com/qbit-ai/qbit/commit/9591ffcdc934641e9af3495b41467dea5d484fd2))
* **ai:** add OpenRouter provider support for arbitrary model IDs ([35eeeb7](https://github.com/qbit-ai/qbit/commit/35eeeb76e5a7c15b25a2557ccc36e97cbbec7375))
* **ai:** add rig-zai crate for Z.AI thinking/reasoning support ([a302916](https://github.com/qbit-ai/qbit/commit/a3029164bc20cf9f1ac00036b509735768b9c1f5))
* **ai:** add web search tools via Tavily integration ([fcd2d73](https://github.com/qbit-ai/qbit/commit/fcd2d730e07f486c9835ca015e8bb7064e225a3b))
* **ai:** add web search tools via Tavily integration ([51f1ba7](https://github.com/qbit-ai/qbit/commit/51f1ba7fd21f180cf8ab5b864fd372101d236e50))
* **ai:** add Z.AI GLM provider support ([5694751](https://github.com/qbit-ai/qbit/commit/5694751772bbece4b108b2605ceea8812c69d8ca))
* **ai:** add Z.AI GLM provider support ([#47](https://github.com/qbit-ai/qbit/issues/47)) ([b5e94f5](https://github.com/qbit-ai/qbit/commit/b5e94f57a3646c87f3c0e1f7e8abc572d45bbedf))
* **ai:** enable all tools for main agent and fix HITL session bug ([9978404](https://github.com/qbit-ai/qbit/commit/99784047a60907f28e400f6c78df2c85ad9b5997))
* **ai:** enable all tools for main agent and fix HITL session bug ([7755f25](https://github.com/qbit-ai/qbit/commit/7755f2543811830ce95d2dd4b761072bf7da2bf3))
* **ai:** enhance reasoning processing and UI for extended thinking mode ([401cd49](https://github.com/qbit-ai/qbit/commit/401cd499ae356d899caf51fd962f31fa3cb7ea0e))
* **ai:** extend AgentBridge and LLM client with Z.AI provider integration ([9543634](https://github.com/qbit-ai/qbit/commit/9543634e59ff041d180a4b86c902b2d76e586421))
* **ai:** implement udiff editing sub-agent ([499a1ea](https://github.com/qbit-ai/qbit/commit/499a1ead8904e544a69f8aaed9b3de08b90f811c))
* **ai:** integrate Z.AI GLM provider with full backend and frontend support ([228272a](https://github.com/qbit-ai/qbit/commit/228272ad64db9063a83f969c257c08ad6d2c2777))
* **ai:** Introduce modular sub-agent execution framework ([df6dc8f](https://github.com/qbit-ai/qbit/commit/df6dc8fdeff9d99ea5dc56d88c9b2f93a67537db))
* **ai:** introduce task planning and management system ([9d7ded8](https://github.com/qbit-ai/qbit/commit/9d7ded8c9ebaf5b284fdf583d0429475432dc85a))
* **ai:** unify provider initialization and enhance multi-provider support ([61d13b6](https://github.com/qbit-ai/qbit/commit/61d13b62183b643e280b097052fc046e89167a34))
* **ai:** wire memory file setting to agent system prompt ([57dbd57](https://github.com/qbit-ai/qbit/commit/57dbd5767901cfd2dc18bec200d0268697fd7283))
* **cli:** implement interactive REPL mode and enhance terminal and JSON output ([e3be1f9](https://github.com/qbit-ai/qbit/commit/e3be1f9a28f7389179c8dbe19b4b0ced568d13a9))
* **context-panel:** add context panel and backend support for enhanced session management ([50fe5f0](https://github.com/qbit-ai/qbit/commit/50fe5f0a99fad85ff1a98862408604f1549b1a58))
* **context:** implement context compaction with end-to-end wiring ([8dd93b8](https://github.com/qbit-ai/qbit/commit/8dd93b8c882bf2059787aacfa78447c345a04b53))
* **evals:** add custom sidecar scorers, utilities, and integration tests ([ee3d256](https://github.com/qbit-ai/qbit/commit/ee3d25612accd9ea972c7cf171b1f92a3dd30f2c))
* **evals:** add DeepEval-based evaluation framework for qbit-cli ([0167435](https://github.com/qbit-ai/qbit/commit/016743535a83f1c5802463e12d757488b74dc00b))
* **evals:** add Rust-native evaluation framework with rig ([b1886cb](https://github.com/qbit-ai/qbit/commit/b1886cb85d01787899c766afe46a3732b77bfa32))
* **evals:** add Rust-native evaluation framework with rig ([c3b443e](https://github.com/qbit-ai/qbit/commit/c3b443eb49ed0b561deb8d23480eaa08c48d2451))
* **evals:** enhance memory recall scenarios and CLI testing framework ([f2fce75](https://github.com/qbit-ai/qbit/commit/f2fce75142aa49f79b119849849a39a667225392))
* **evals:** introduce Layer 1 session state support with scorers, utilities, and API types ([cd8ee8f](https://github.com/qbit-ai/qbit/commit/cd8ee8f80b14bb162017b7e8aa3531e2995b9ca0))
* **evals:** Rust-native evaluation framework with rig ([4fd37f2](https://github.com/qbit-ai/qbit/commit/4fd37f2bbd9af061b5d9c1f30ec9709477d3771c))
* **frontend:** add migrateCodebaseIndex wrapper ([3cb6903](https://github.com/qbit-ai/qbit/commit/3cb690337914639b35c48281cad5f9f063f1eb31))
* **indexer:** add codebase management commands ([7f328ff](https://github.com/qbit-ai/qbit/commit/7f328ff6849eca7fcc487cbfb388907c26182872))
* **indexer:** add configurable global index storage location ([a647877](https://github.com/qbit-ai/qbit/commit/a647877550b80eb65dac2a0a1111af8b5044ecd3))
* **indexer:** add paths module for index directory resolution ([6988a0b](https://github.com/qbit-ai/qbit/commit/6988a0bf2885a8d347e785557c83664520f30806))
* **indexer:** integrate configurable storage location ([10d2bfb](https://github.com/qbit-ai/qbit/commit/10d2bfbbad175066ce13a4366721e18e1fdfb5ac))
* **input:** add @ file reference commands for agent mode ([b0a7648](https://github.com/qbit-ai/qbit/commit/b0a7648cdf9eec3cf3549d88b84dd4fd94369c9f))
* **input:** improve path completion with final selection handling ([bb031eb](https://github.com/qbit-ai/qbit/commit/bb031eb1d835f87fea600351ef73dc685e980769))
* **input:** integrate @ file commands into UnifiedInput ([9477071](https://github.com/qbit-ai/qbit/commit/94770712c2cf2f1bc4380d9fbe36bd1bcb23cfc9))
* **mock-devtools:** implement incremental diffs, baselines, and context improvements ([7e2e500](https://github.com/qbit-ai/qbit/commit/7e2e500dfab98ccf6bc5899d1a058e97591731b7))
* **mock-devtools:** implement incremental diffs, baselines, and context improvements ([696f93b](https://github.com/qbit-ai/qbit/commit/696f93b0a81d3fe6f269884abfd8f8c11d199305))
* **models:** update Gemini and Groq model lists ([30aa25a](https://github.com/qbit-ai/qbit/commit/30aa25a04a42a46fa73a74383a3de74eb3e00e89))
* **models:** update model lists and defaults for Gemini, Groq, and xAI ([bca43aa](https://github.com/qbit-ai/qbit/commit/bca43aac3ff2462efab6ecebd94eca453f3c6628))
* per-session AI agent isolation ([69a3bc5](https://github.com/qbit-ai/qbit/commit/69a3bc5510b9ef75396d59aaaffbb209bc705ef9))
* **pty:** detect alternate screen buffer via ANSI CSI sequences ([29e77b4](https://github.com/qbit-ai/qbit/commit/29e77b40478c69d5abc7866b9cfc21e711043883))
* register workflow commands in Tauri app ([04679b8](https://github.com/qbit-ai/qbit/commit/04679b8d208b55c648ef0925db3fa05b8c5e3305))
* **rig-zai:** add custom streaming with reasoning_content support ([5abf6e5](https://github.com/qbit-ai/qbit/commit/5abf6e5f8ff5a3815ca8e6c2c42607481169d3dd))
* **rig-zai:** enable thinking mode for GLM-4.7 ([4a3984f](https://github.com/qbit-ai/qbit/commit/4a3984f0b62c9eb926bffe0121c5d46f36f73727))
* **runtime:** abstract event emission with runtime and CLI support ([afd5b51](https://github.com/qbit-ai/qbit/commit/afd5b5105ef3574104c685b6816e968a9761937d))
* **runtime:** enable event emission support with Tauri integration and enhanced Layer 1 logging ([4bdd872](https://github.com/qbit-ai/qbit/commit/4bdd872a0765417b3b96560e3ad454d2a75e1a50))
* **server:** add HTTP/SSE server support for CLI and evaluation framework ([c399149](https://github.com/qbit-ai/qbit/commit/c399149f051f265e1363176304d04922e6c5e3c8))
* **settings:** add CodebaseConfig schema for codebase management ([d742755](https://github.com/qbit-ai/qbit/commit/d74275588f0ae330dc716cb33ea9fc34ba83e091))
* **settings:** add fullterm_commands setting for custom TUI apps ([336c09f](https://github.com/qbit-ai/qbit/commit/336c09f52094231d042bc613b1cbd63af75e98ac))
* **settings:** add IndexLocation enum for configurable index storage ([2aa7b35](https://github.com/qbit-ai/qbit/commit/2aa7b354624dc69b2c8401d314f4c1c02971bd84))
* **settings:** add provider visibility toggle for model selector ([de402f6](https://github.com/qbit-ai/qbit/commit/de402f6d033b87578e9bfc90fd5726f54a471f1c))
* **settings:** add provider visibility toggle UI ([15fe28b](https://github.com/qbit-ai/qbit/commit/15fe28b95c32e403c9f0666081d07133cf49f903))
* **settings:** add settings system with UI and settings.toml ([8edfed7](https://github.com/qbit-ai/qbit/commit/8edfed7ed21a675cd6b8b153d34548a2533e4f30))
* **settings:** add show_in_selector field to AI provider settings ([b441114](https://github.com/qbit-ai/qbit/commit/b441114a05a87169107b1a29a6fc6545ee60eddb))
* **shell:** add multi-shell support for bash and fish ([e629585](https://github.com/qbit-ai/qbit/commit/e629585828679e75cf69ce23c34f550c9b769370))
* **shell:** add venv reporting to shell integration scripts ([03ace38](https://github.com/qbit-ai/qbit/commit/03ace38ac0d4b25b8eed9080a6c6c4714446c656))
* **sidecar:** add context capture system for session tracking ([07cfa37](https://github.com/qbit-ai/qbit/commit/07cfa37615020f1fe3f27d7420c3adb5b56ff807))
* **sidecar:** add context capture system for session tracking ([b5105bb](https://github.com/qbit-ai/qbit/commit/b5105bba3bac855a32c23586900f593d66bd9c92))
* **sidecar:** add optional `local-llm` feature for mistral.rs integration ([4078d03](https://github.com/qbit-ai/qbit/commit/4078d03041b14701fe9cdfd2dafcc15d68c4355b))
* **sidecar:** add session resume and matching functionality to enhance context restoration ([591d9ab](https://github.com/qbit-ai/qbit/commit/591d9abe5c4d7c2ecfacc6dafeb598ea10208597))
* **sidecar:** add session resume and matching functionality to enhance context restoration ([18cac0a](https://github.com/qbit-ai/qbit/commit/18cac0ad03d99464d9e27bde85d063fb4acead84))
* **sidecar:** enhance context panel with patches and artifacts integration ([2f62e07](https://github.com/qbit-ai/qbit/commit/2f62e07e417e0a7b6073833d6e5d7907828d70e7))
* **sidecar:** enhance LLM-based state management and context panel UI ([ffb8aca](https://github.com/qbit-ai/qbit/commit/ffb8acaf07a319c53e0a7ab82744beace85289f2))
* **sidecar:** enhance synthesis metadata, context panel, and settings ([863daf8](https://github.com/qbit-ai/qbit/commit/863daf8db67b5f288799d9bfdac3fc0e0bf7c1d7))
* **sidecar:** expand session diagnostics and enhance GCP token handling ([9cdbc83](https://github.com/qbit-ai/qbit/commit/9cdbc83e75915461c409c057c1882897254a50d9))
* **sidecar:** implement LLM-based commit message generation ([bc58b2e](https://github.com/qbit-ai/qbit/commit/bc58b2ecf6f4bc669aa253f97d5aaaf4b9dc1311))
* **sidecar:** introduce schema verification tests and embeddings support ([c72a362](https://github.com/qbit-ai/qbit/commit/c72a36256be34e0a64b9071077dbe4351b41c666))
* **sidecar:** remove session_start events and extend event schema ([45678df](https://github.com/qbit-ai/qbit/commit/45678df2b88b49d46e3b912591bd7038c35ac77f))
* **statusbar:** filter model selector based on provider visibility ([c677c81](https://github.com/qbit-ai/qbit/commit/c677c8107503fab76f3131e48e9daa74161ca3a0))
* **store:** add renderMode state for terminal display modes ([d0d2b18](https://github.com/qbit-ai/qbit/commit/d0d2b18d0e8ee0881175448f16fab45752301a8b))
* **tabs:** customizable tab names and process display ([1be50ec](https://github.com/qbit-ai/qbit/commit/1be50ecf81b615b0178c64382d594842fb479aed))
* **tabs:** customizable tab names and process display ([259f337](https://github.com/qbit-ai/qbit/commit/259f33702a4e8f0f1628dfbcb6449b35a46828c8))
* **terminal:** add DEC 2026 synchronized output and improve TUI compatibility ([#48](https://github.com/qbit-ai/qbit/issues/48)) ([21b3cfd](https://github.com/qbit-ai/qbit/commit/21b3cfd743962d051bf07016e60ef2f0cd0cd550))
* **terminal:** add fullterm mode for interactive CLI apps ([016b1d7](https://github.com/qbit-ai/qbit/commit/016b1d724772e8d67f69c8fa8ebd8903e5796fa8))
* **terminal:** add fullterm mode with auto-switch for interactive commands ([5b735dc](https://github.com/qbit-ai/qbit/commit/5b735dc5355ed7e7f99783dbc756554a907f0c01))
* **terminal:** add virtual environment detection and display ([6dc5292](https://github.com/qbit-ai/qbit/commit/6dc5292c787702725c32d3ae4109fc1c45c013aa))
* **terminal:** add VirtualTerminal for ANSI sequence processing ([a87f16a](https://github.com/qbit-ai/qbit/commit/a87f16a70c9e31c617f0ae9652c9ebee53f43ccc))
* **terminal:** add VirtualTerminalManager and useProcessedOutput hook ([f8bc50e](https://github.com/qbit-ai/qbit/commit/f8bc50ed8e5fe1b4c289e31d6fbdffd74cc4654e))
* **terminal:** integrate VirtualTerminal for pending command output ([58b760f](https://github.com/qbit-ai/qbit/commit/58b760f0ba49eea9e12d8e2301392a0331931d14))
* **themify-ui:** extend theme tokens to more ui components ([37cddc8](https://github.com/qbit-ai/qbit/commit/37cddc8f90a3c8e531ca919c3287f85fef8b2992))
* **themify-ui:** extend theme tokens to more ui components ([fdf97d2](https://github.com/qbit-ai/qbit/commit/fdf97d262afab3aa2fdf8cd04cf183f645ea574c))
* **theming:** add theme support ([bcce8bc](https://github.com/qbit-ai/qbit/commit/bcce8bce7cb49aaefc95dd7ccd046713c58ec58f))
* **ui:** add accessibility labels to input mode toggle buttons and implement input focus e2e tests ([3c3b878](https://github.com/qbit-ai/qbit/commit/3c3b878e83ba22daa82265f1e4fc5b37e8490ad2))
* **ui:** add Codebases settings tab for managing indexed repos ([a1cddfd](https://github.com/qbit-ai/qbit/commit/a1cddfd01dd88bea911fe9276c2ee991409ed77c))
* **ui:** add Codebases settings tab for managing indexed repositories ([81d30e0](https://github.com/qbit-ai/qbit/commit/81d30e0c4b775737222cb3e2b039a8c01143c138))
* **ui:** add copy button to markdown code blocks ([862d27f](https://github.com/qbit-ai/qbit/commit/862d27f1c4922c163f5b7ede469ff84cb24c8021))
* **ui:** add ctrl+R reverse history search ([96427b9](https://github.com/qbit-ai/qbit/commit/96427b97eb6494daf025c211581f7134a9a7ce84))
* **ui:** add diff view for edit_file tool results ([999044e](https://github.com/qbit-ai/qbit/commit/999044e8f204b60ad727e50efc0fec940d488d88))
* **ui:** add fullterm mode toggle and status indicator ([7dfb978](https://github.com/qbit-ai/qbit/commit/7dfb9786b75f7c665534511cb708558855fa7d32))
* **ui:** add OpenAI provider to frontend ([205e8bc](https://github.com/qbit-ai/qbit/commit/205e8bc7ee50e71d72657a1ed5811f235275b6b8))
* **ui:** add OpenRouter model selection to StatusBar and Settings ([a19fc35](https://github.com/qbit-ai/qbit/commit/a19fc35f61fae17e2964fa745f9a2fdedfe0e945))
* **ui:** add slash commands for user-defined prompts ([e730573](https://github.com/qbit-ai/qbit/commit/e730573eb9aeb0982f8061b31ee178c081f5d031))
* **ui:** add slash commands for user-defined prompts ([ae55346](https://github.com/qbit-ai/qbit/commit/ae5534694af8885adb81a9a41474529bcac4daa2))
* **ui:** add sub-agent tool call details display ([92121f5](https://github.com/qbit-ai/qbit/commit/92121f5e1a7e4e9f1d62472ad27c2c94c612a559))
* **ui:** add task planner panel and status bar integration ([0285ba8](https://github.com/qbit-ai/qbit/commit/0285ba8d8864d5b5ac436e043f22af9fc501f80d))
* **ui:** add terminal mode indicator to status bar ([15835b0](https://github.com/qbit-ai/qbit/commit/15835b0152b19db1d532dc7247000c65b63e30f7))
* **ui:** add terminal mode indicator to status bar ([20f9fe8](https://github.com/qbit-ai/qbit/commit/20f9fe89cc6bd9673d94adce5559f6327817d719))
* **ui:** add tool call details modal ([260312a](https://github.com/qbit-ai/qbit/commit/260312a325a91d486ba0c0eb1a5b03d93fbabf13))
* **ui:** add workflow UI components ([b0f26ed](https://github.com/qbit-ai/qbit/commit/b0f26ed6bed8428adcea3a0c15c321113c49a9c4))
* **ui:** add WorkflowTree component for hierarchical display ([0c2cf3e](https://github.com/qbit-ai/qbit/commit/0c2cf3e2ab73e7d02429ea0cd2e809f70a088c86))
* **ui:** display git branch in status bar ([#55](https://github.com/qbit-ai/qbit/issues/55)) ([a7c1c52](https://github.com/qbit-ai/qbit/commit/a7c1c526989a39f5beee3b839d6503c0c94ae44d))
* **ui:** enhance tool group and AI workflow integration ([72817fa](https://github.com/qbit-ai/qbit/commit/72817fa83c8c7b3b8e2f29215f891fa3c095825a))
* **ui:** implement native macOS titlebar with draggable region ([194fb6f](https://github.com/qbit-ai/qbit/commit/194fb6f917b20b68ff97939806b00d0e4a4a685b))
* **ui:** implement native macOS titlebar with draggable region ([05f88d5](https://github.com/qbit-ai/qbit/commit/05f88d5eee6da7cf9af9b4c48a34af3322e3677a))
* **ui:** integrate workflow system into application ([915c18a](https://github.com/qbit-ai/qbit/commit/915c18aa8912bf828f5e7a014921b5db72202566))
* **workflow:** add core workflow infrastructure ([2ed0f01](https://github.com/qbit-ai/qbit/commit/2ed0f01f4854aea0b0618fab1f85c5c8f094d54b))
* **workflow:** add git commit workflow agents ([ce31a55](https://github.com/qbit-ai/qbit/commit/ce31a55e22f1cd16ace30d99bd90ed6731abc226))
* **workflow:** add Tauri workflow commands ([8cab27c](https://github.com/qbit-ai/qbit/commit/8cab27cb171c3d53d674baf1120b7dca32d3c4f8))
* **workflow:** integrate workflow system with AI module ([d28833a](https://github.com/qbit-ai/qbit/commit/d28833a6f9129fa9a432e218054ebc9efe3bfa78))


### Bug Fixes

* add packages field to pnpm-workspace.yaml ([3e64c3b](https://github.com/qbit-ai/qbit/commit/3e64c3b7e342330d11a65d5badda9ea8cdf0c09c))
* **ai:** use camelCase for Tauri invoke parameters in session-specific commands ([7296b5d](https://github.com/qbit-ai/qbit/commit/7296b5d50a1a92014576dcec9e521adad128bd96))
* allow dead_code for unused HunkApplyError variant ([4e6d39d](https://github.com/qbit-ai/qbit/commit/4e6d39d2b48096aec10ac41d335ebc7c5d57c2d2))
* **app:** use function call for browser mode detection ([55cdbdb](https://github.com/qbit-ai/qbit/commit/55cdbdbe24515159374a756533d777a5bc8610f4))
* **ci:** make sccache gracefully fallback when unavailable ([0da1e88](https://github.com/qbit-ai/qbit/commit/0da1e88bc733e3880c05263c11a36a33aed8eeda))
* **ci:** remove pnpm caching to fix store path error ([28691b1](https://github.com/qbit-ai/qbit/commit/28691b1096ed6b39cbcae294b554cff40c0a3ad1))
* **ci:** resolve illegal path in release-please config ([#59](https://github.com/qbit-ai/qbit/issues/59)) ([d361bcc](https://github.com/qbit-ai/qbit/commit/d361bcca71546cbad9638d21670c78d08ba02159))
* **ci:** simplify release-please config for monorepo ([aaa1e3e](https://github.com/qbit-ai/qbit/commit/aaa1e3e1087350676a672a52ed1541ef6e312242))
* **ci:** simplify release-please config for monorepo ([e9ed088](https://github.com/qbit-ai/qbit/commit/e9ed08809dff5abc0ba79dab51cd5972115634f7))
* **ci:** update evals workflow for Rust evals framework ([f3442be](https://github.com/qbit-ai/qbit/commit/f3442bee571325c9317cc3e83d63c64b4ffddea4))
* **ci:** use built-in pnpm caching in setup-node ([d3dabde](https://github.com/qbit-ai/qbit/commit/d3dabde6b8e8ddff0c37ae93eb6f4123ffe2d6fb))
* correct command_block event format for terminal output ([0385546](https://github.com/qbit-ai/qbit/commit/03855466571b355ab8b54befa1d5a87965eaef31))
* **deps:** remove unused lancedb and vector DB dependencies ([e3acd83](https://github.com/qbit-ai/qbit/commit/e3acd83e8d0262b3a373a7ad07f33517164bfe11))
* displaying shell and ai responses ([04d5f62](https://github.com/qbit-ai/qbit/commit/04d5f62dc495a16885c1176f7db0b9192d405841))
* displaying shell and ai responses ([645007c](https://github.com/qbit-ai/qbit/commit/645007ce9fb5813efed846ec92a2579cae2cb185))
* **e2e:** add Z.AI provider to mock settings ([e6f3aa8](https://github.com/qbit-ai/qbit/commit/e6f3aa8495ec04f7b4ae9d295b8290f2ec12985b))
* **e2e:** clear notifications during test setup ([b55ed6e](https://github.com/qbit-ai/qbit/commit/b55ed6e8baeebecede782e69afc37bc7fdfce353))
* **e2e:** fix test locators and accessibility issues ([884c88f](https://github.com/qbit-ai/qbit/commit/884c88f24212965c625afc31aacc069edb89cf82))
* **e2e:** improve test reliability by waiting for app readiness ([9660326](https://github.com/qbit-ai/qbit/commit/966032696ec83de37f223fcf0f6c033ebe7757d5))
* **e2e:** replace waitForTimeout with auto-retrying assertions ([ffac557](https://github.com/qbit-ai/qbit/commit/ffac557b2776c9f38b2d3966557324da9f6fa4cf))
* **e2e:** use role-based dialog selector to avoid strict mode violation ([b5e8968](https://github.com/qbit-ai/qbit/commit/b5e89682fda5fa64dff01534e60c0980a1aba251))
* **frontend:** add Z.AI provider to StatusBar model selector ([49f36c7](https://github.com/qbit-ai/qbit/commit/49f36c7e274376df98216a04879ac1f55468aad4))
* **frontend:** use session working directory for AI agent initialization ([948bd21](https://github.com/qbit-ai/qbit/commit/948bd212923e366e881659ed5dcf083b4a7218de))
* handle plugin:event IPC commands in mocks ([2cd86ad](https://github.com/qbit-ai/qbit/commit/2cd86adcd15f3969ef137b5c48bc1df62c6ee155))
* implement proper event dispatching for mock system ([ac2fccb](https://github.com/qbit-ai/qbit/commit/ac2fccb053974d07605799ad06055389748c9bff))
* make mock event system work with ES module restrictions ([0b769d1](https://github.com/qbit-ai/qbit/commit/0b769d12dc3cf1c10fb1d7a5040030f667ae494b))
* **mocks:** return valid mock credentials for Vertex AI config ([9ae6115](https://github.com/qbit-ai/qbit/commit/9ae6115b411b331506f9799b72faf1bedc0e2bb7))
* **models:** update Anthropic models to Claude 4.5 and use constants ([0dbf35b](https://github.com/qbit-ai/qbit/commit/0dbf35bb2afeef8594f9620cb6f586587a4d4bb2))
* resolve clippy warnings for CI ([ce5f5fb](https://github.com/qbit-ai/qbit/commit/ce5f5fbb7958369f2f63f5bc8e7d4d4e5e6687cb))
* resolve IPv6 localhost issue for Playwright tests ([aa4e5a4](https://github.com/qbit-ai/qbit/commit/aa4e5a45fd4ac61a375de62a40beb2fb09de5ea8))
* resolve lint errors for CI checks ([816a183](https://github.com/qbit-ai/qbit/commit/816a18352c0553209d04bc3f2b72d4eeb7d33878))
* resolve test failures after sub-agent merge ([7efb6ee](https://github.com/qbit-ai/qbit/commit/7efb6ee0e1e177be79d9d5cb0177499c8bab478b))
* resolve test failures and improve test stability ([84110d3](https://github.com/qbit-ai/qbit/commit/84110d369ee61f5e070cada9c01385d80c2a9808))
* **rig-zai:** add budget_tokens and debug logging for thinking mode ([9a0fac2](https://github.com/qbit-ai/qbit/commit/9a0fac25e05d271d2c416bd9bba35ac86a8c4c66))
* **settings:** preserve codebase configs when saving settings ([21495d5](https://github.com/qbit-ai/qbit/commit/21495d5c90fabf2731102e149f3c3c92eb7474fe))
* **settings:** resolve fullscreen dialog layout and overflow issues ([d44a513](https://github.com/qbit-ai/qbit/commit/d44a513fa666d74599ee7c87dcd2d0d2d1ec14c2))
* **settings:** resolve fullscreen dialog layout and overflow issues ([c0d63fa](https://github.com/qbit-ai/qbit/commit/c0d63fa9ab069cadb924fae2f1cd66db883db690))
* **store:** refine Vertex AI provider validation and enhance TypeScript checks ([3ddac7e](https://github.com/qbit-ai/qbit/commit/3ddac7e138ccd45d377ba1ddf65878099e8a8210))
* **store:** skip command block creation in fullterm mode ([5310df8](https://github.com/qbit-ai/qbit/commit/5310df849ea6ea118695de25e8e0788c4a5b0e9e))
* **tabs:** allow closing the last tab ([022cf91](https://github.com/qbit-ai/qbit/commit/022cf91e9894ea880fd38e9c8bd4f0d439f4e6c5))
* terminal input focus ([6e89d95](https://github.com/qbit-ai/qbit/commit/6e89d95f2300a5f4a14e40b29b8364cf54f40ba0))
* **tools:** improve error messages for file path resolution ([e45d2c2](https://github.com/qbit-ai/qbit/commit/e45d2c245a9ac2dbb5e87acd56e9f720c35da393))
* **ui:** add min-h-0 to ContextPanel flex containers for proper scrolling ([863d0ba](https://github.com/qbit-ai/qbit/commit/863d0ba6283002a268304e37108d3f4c95d2a8de))
* **ui:** align streaming and completed agent response font styles ([ddde5b4](https://github.com/qbit-ai/qbit/commit/ddde5b4138b65bead3d10e783deec33169ee8e67))
* **ui:** align streaming and completed agent response font styles ([29ce21c](https://github.com/qbit-ai/qbit/commit/29ce21c365680de710292a798a46787883e883ba))
* **ui:** extend tool cards to full width like thinking cards ([39ee63f](https://github.com/qbit-ai/qbit/commit/39ee63f26fc612367e6563da9d490acd53545e92))
* **ui:** extend tool cards to full width like thinking cards ([3fe7b11](https://github.com/qbit-ai/qbit/commit/3fe7b110e1edaba361a8c1e4b864e19056e410a7))
* **ui:** reset input submission state when switching sessions ([586ef39](https://github.com/qbit-ai/qbit/commit/586ef399686802b98455c3c02f756f464395609c))
* **ui:** reset input submission state when switching sessions ([143aa2e](https://github.com/qbit-ai/qbit/commit/143aa2ef418b8cc58a6af7c353db9b07bbac7f94))
* **ui:** terminal input focus ([2fbbf44](https://github.com/qbit-ai/qbit/commit/2fbbf4458e324a204c5dab8aa7e5021f708d45d2))
* update CLI bootstrap for new sidecar API and add sidecar evals ([2a97aa7](https://github.com/qbit-ai/qbit/commit/2a97aa774fc11923301b092d819769166e2c99bb))


### Performance

* **ci:** add sccache and improve cargo caching for evals ([7f73747](https://github.com/qbit-ai/qbit/commit/7f73747d1324c00665df84277d72578a592dbbe3))
* **ci:** add sccache to check workflow ([8b3d3f0](https://github.com/qbit-ai/qbit/commit/8b3d3f046a7b39d8ba901b64567198f67e0ca504))
* **ci:** use debug build for evals (faster compile, network-bound runtime) ([8501459](https://github.com/qbit-ai/qbit/commit/8501459a32055dd46c7a06bedd37b7a76888bfa8))


### Refactoring

* add `#[allow(dead_code)]` for test-only functions and metadata ([dce7fa2](https://github.com/qbit-ai/qbit/commit/dce7fa2ed9340a9a9d71251ab32615c4f54267e9))
* add `#[allow(dead_code)]` to public API functions and structs ([c39b8e2](https://github.com/qbit-ai/qbit/commit/c39b8e2a70d9b0e33db87201eed3c2baa72dbe62))
* **agent-chat:** separate sub-agent and content blocks for improved rendering ([f9179d8](https://github.com/qbit-ai/qbit/commit/f9179d83dafe227be14729c45edf2f15f89c9b98))
* **ai, ui:** enhance Markdown rendering, sub-agent management, and streaming handling ([b1ac064](https://github.com/qbit-ai/qbit/commit/b1ac064a09b2e1047990862986e5ec4a4586fcfa))
* **ai:** Adjust defaults and improve error handling in agentic loop ([1d9183e](https://github.com/qbit-ai/qbit/commit/1d9183ee4a378794af8f0f80218130c1088f85ab))
* **ai:** improve code structure and reuse across modules ([c89f70f](https://github.com/qbit-ai/qbit/commit/c89f70f340ce6d07652ab3242761ae59135926f1))
* **ai:** remove PromptContext and simplify prompt handling ([d6697f4](https://github.com/qbit-ai/qbit/commit/d6697f41745fa4b80c7850463a130cef937ed4bc))
* **ai:** remove unused is_default method from AgentMode ([5271b1e](https://github.com/qbit-ai/qbit/commit/5271b1edb1d1bd99cdaf4f77197dfcf054704e93))
* **ai:** remove unused methods and tests, simplify handling across modules ([de323ab](https://github.com/qbit-ai/qbit/commit/de323abaf3f88428c2d79ae42b6fc839ff20149c))
* **ai:** reorganize commands module into logical submodules ([861fa7e](https://github.com/qbit-ai/qbit/commit/861fa7e49337124e56af062c874fd4e0c12165a8))
* **cli:** remove indexer initialization from CLI bootstrap ([ac55ca8](https://github.com/qbit-ai/qbit/commit/ac55ca8c08f0ded59afa7b32abc49b6411bac68f))
* **cli:** simplify `CliRuntime::new` invocation and remove redundant newline in `session.rs` ([8aea922](https://github.com/qbit-ai/qbit/commit/8aea922989b66995308dd30942a52ed98edb9ad8))
* **CommandPalette, UnifiedInput:** Simplify mode handling with toggle logic ([df6dc8f](https://github.com/qbit-ai/qbit/commit/df6dc8fdeff9d99ea5dc56d88c9b2f93a67537db))
* **dependencies:** reorder imports in qbit modules for consistency ([63fc1fc](https://github.com/qbit-ai/qbit/commit/63fc1fce4c3bc4f1723b614c6b964e122ffb7632))
* **eval:** simplify server handling and allow configurable workspace via env variable ([b38f5fc](https://github.com/qbit-ai/qbit/commit/b38f5fc108b5fb0ac576a53bdf984d75e81d1e16))
* Extract Rust backend into modular workspace crates ([#50](https://github.com/qbit-ai/qbit/issues/50)) ([37bffd1](https://github.com/qbit-ai/qbit/commit/37bffd184d26a7057976c8a060a010c5fa55d547))
* **frontend:** improve ANSI fallback and simplify UI ([5149c91](https://github.com/qbit-ai/qbit/commit/5149c9111502d76f2d3a29d7800e45acf052eacd))
* **frontend:** remove auto-indexing from app initialization ([1c39361](https://github.com/qbit-ai/qbit/commit/1c39361313a63e9088555105b5ff46b9946a4505))
* **frontend:** use ANSI-based fullterm mode detection ([4cc635e](https://github.com/qbit-ai/qbit/commit/4cc635ec8622167e5e61d153f456024dbc7451fc))
* Improve code readability, formatting, and AI workspace syncing ([96f489d](https://github.com/qbit-ai/qbit/commit/96f489d6c0750b0596187e6dcd0f2994ca4473e6))
* **logging:** enhance tracing for tool execution, session management, and PTY operations ([3c8d6eb](https://github.com/qbit-ai/qbit/commit/3c8d6ebe0c8cbf94e2b77aa55affed68d261cdc8))
* **mocks:** simplify `validateRequiredParams` function signature for cleaner readability ([25e0651](https://github.com/qbit-ai/qbit/commit/25e0651a00ddd4327187c0ee67256485c5a89be1))
* **models:** consolidate model definitions and simplify accessors ([ca9ea66](https://github.com/qbit-ai/qbit/commit/ca9ea66c9366019309bfb2905425d1c5d3dc2f75))
* optimize imports, formatting, and minor logic updates ([1281a9a](https://github.com/qbit-ai/qbit/commit/1281a9a86a1cea0557006e4b2d9c86100b6f10a9))
* **pty/manager:** prioritize `QBIT_WORKSPACE` for working directory resolution ([bc58b2e](https://github.com/qbit-ai/qbit/commit/bc58b2ecf6f4bc669aa253f97d5aaaf4b9dc1311))
* remove deprecated code and streamline API across core, cli, and ui ([8295346](https://github.com/qbit-ai/qbit/commit/8295346168d415abbb763d4200062a19f9e5c194))
* remove old monolithic workflow module ([70877b4](https://github.com/qbit-ai/qbit/commit/70877b4019191bfb87c5f478fda6c7986471d633))
* remove unused code and improve modularization across components ([8592008](https://github.com/qbit-ai/qbit/commit/85920085c338204ef0d84bcf6c0306a9daa40850))
* remove unused test cases and obsolete functions ([0e3bf7d](https://github.com/qbit-ai/qbit/commit/0e3bf7d548bba0923955db54efb24d3c0e634676))
* rename project directories for clarity (src-tauri→backend, src→frontend) ([97c01e9](https://github.com/qbit-ai/qbit/commit/97c01e9ca886e65fbaccc1dd00b24b694680ed54))
* rename src-tauri to backend and src to frontend ([83ed990](https://github.com/qbit-ai/qbit/commit/83ed990ae410b4dbdab1926365fef62132f65b89))
* **rig-zai:** simplify tool call handling and improve OpenAI compatibility ([ededc4e](https://github.com/qbit-ai/qbit/commit/ededc4e516be272973df9064e718e9d505d106a2))
* **rig-zai:** simplify tool call handling and improve OpenAI compatibility ([e7352b2](https://github.com/qbit-ai/qbit/commit/e7352b24d58c11f543acf488becfd5d0de4b19f4))
* **rust:** implement high-impact simplifications from rust-simplifier review ([f70e7e6](https://github.com/qbit-ai/qbit/commit/f70e7e6a93276f05c5b16f65636af518580a20a6))
* **sidecar:** make session management atomic and add idempotency tests ([9d8459d](https://github.com/qbit-ai/qbit/commit/9d8459ddfaee23f4611a3a56fcccac6d3bacc0b2))
* **sidecar:** replace LanceDB architecture with markdown-based sessions ([6813ab6](https://github.com/qbit-ai/qbit/commit/6813ab6da4850cde8ded20735a06b03e8ad66044))
* **sidecar:** replace LanceDB with markdown-based sessions ([b7f1a62](https://github.com/qbit-ai/qbit/commit/b7f1a62f8ebd33556d043892f0f6cceb56377d7c))
* **sidecar:** simplify session architecture and improve patch handling ([46e24e6](https://github.com/qbit-ai/qbit/commit/46e24e6af9afe28fbef66b9f59ee5195454f504b))
* **sidecar:** simplify session architecture and improve patch handling ([a7aa130](https://github.com/qbit-ai/qbit/commit/a7aa130a21083c737c4c2fea8a651de0efa7f0d0))
* simplify and reorganize agent evaluation tests ([5411847](https://github.com/qbit-ai/qbit/commit/5411847ad9626c426b08a01ae973fd4153e0e72f))
* **terminal:** add barrel export for Terminal component ([9713d87](https://github.com/qbit-ai/qbit/commit/9713d875244473b4859fde3b9abb0bf55df4f28c))
* **tests, workspace:** overhaul session and file operation tests; cleanup unused fixtures ([ed0769f](https://github.com/qbit-ai/qbit/commit/ed0769f180b1eca9a6175622f8e2a0592558d99f))
* **tests:** enhance batch prompt execution logging and verbose mode handling ([d26198b](https://github.com/qbit-ai/qbit/commit/d26198ba37ac91b23754e16cda0038684af93fed))
* **tests:** remove unused `test_events_jsonl_created` function from `test_sidecar.py` ([f20090e](https://github.com/qbit-ai/qbit/commit/f20090e7edb2f6eec24b511478fe4bd924835ae0))
* **tests:** replace `networkidle` with `domcontentloaded` in page load waits for e2e tests ([c618b1e](https://github.com/qbit-ai/qbit/commit/c618b1e820b03b4bfc3cf33e3c6abd3364a7fe82))
* **tests:** replace `networkidle` with `domcontentloaded` in page load waits for e2e tests ([a2f4fef](https://github.com/qbit-ai/qbit/commit/a2f4fef25195be5985ad075ff7f24e265dfce781))
* **theme:** replace hardcoded colors with CSS variables and improve component styles ([dce8f67](https://github.com/qbit-ai/qbit/commit/dce8f673d8088ab3401f226fa6add98da4395ae0))
* **tool-display:** replace inline expansion with modal details view ([dc00e77](https://github.com/qbit-ai/qbit/commit/dc00e777e46759da40b4d09953b9af4c3d0b33b6))
* **tool-display:** replace inline expansion with modal details view ([de86e26](https://github.com/qbit-ai/qbit/commit/de86e269cd891a2116427c45b526d0b47125cf09))
* **tools:** migrate from vtcode-core to qbit-tools ([f8a3c9e](https://github.com/qbit-ai/qbit/commit/f8a3c9ec871ae0072f57e0072a9eab80c811ef7e))
* UI overhaul with shadcn components, added ComponentTestbed, and updated dependencies for improved modularity. ([840eac6](https://github.com/qbit-ai/qbit/commit/840eac6cef54cf507506081fb098d27e73b873ed))
* **ui, ai:** improve code sharing and clean up deprecated components ([c297c74](https://github.com/qbit-ai/qbit/commit/c297c74528d7b4805d312db58125f7f7b7e592d3))
* **ui:** adjust left margin and border styles for improved layout consistency ([7ec7876](https://github.com/qbit-ai/qbit/commit/7ec7876264d97ef3d7bf5e98767fa563e1d75a22))
* **ui:** enhance styles and improve component readability ([5165822](https://github.com/qbit-ai/qbit/commit/51658223dd0d2867fae32b3a9c8f44f3a1b1c417))
* **ui:** simplify `CommandBlock` styles and remove unused components ([f25d3d1](https://github.com/qbit-ai/qbit/commit/f25d3d1e832146149086925156a2c823de8badec))
* **ui:** simplify `WelcomeScreen` by removing unused sub-agent and workflow capabilities logic ([70f4df5](https://github.com/qbit-ai/qbit/commit/70f4df5132f1b57fec67770cf14c9b0b804dc893))
* vtcode migration part 1 - dead code cleanup and modularization ([a8919f9](https://github.com/qbit-ai/qbit/commit/a8919f9edf7bc72c80d20ba2c7aef593ea75c9e1))
