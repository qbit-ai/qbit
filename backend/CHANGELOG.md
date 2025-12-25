# Changelog

## [0.2.0](https://github.com/qbit-ai/qbit/compare/qbit-v0.1.0...qbit-v0.2.0) (2025-12-25)


### Features

* add conversation-level token usage tracking ([#49](https://github.com/qbit-ai/qbit/issues/49)) ([ac21420](https://github.com/qbit-ai/qbit/commit/ac214209d6e846540b835485f579fba50deab170))
* **ai:** add dynamic memory file lookup from settings ([e1ffaec](https://github.com/qbit-ai/qbit/commit/e1ffaec2434a9b8a825d932ca1a6c2e9c4668f4e))
* **ai:** add rig-zai crate for Z.AI thinking/reasoning support ([a302916](https://github.com/qbit-ai/qbit/commit/a3029164bc20cf9f1ac00036b509735768b9c1f5))
* **ai:** add Z.AI GLM provider support ([#47](https://github.com/qbit-ai/qbit/issues/47)) ([b5e94f5](https://github.com/qbit-ai/qbit/commit/b5e94f57a3646c87f3c0e1f7e8abc572d45bbedf))
* **ai:** implement udiff editing sub-agent ([499a1ea](https://github.com/qbit-ai/qbit/commit/499a1ead8904e544a69f8aaed9b3de08b90f811c))
* **ai:** wire memory file setting to agent system prompt ([57dbd57](https://github.com/qbit-ai/qbit/commit/57dbd5767901cfd2dc18bec200d0268697fd7283))
* **evals:** add Rust-native evaluation framework with rig ([b1886cb](https://github.com/qbit-ai/qbit/commit/b1886cb85d01787899c766afe46a3732b77bfa32))
* **evals:** add Rust-native evaluation framework with rig ([c3b443e](https://github.com/qbit-ai/qbit/commit/c3b443eb49ed0b561deb8d23480eaa08c48d2451))
* **evals:** Rust-native evaluation framework with rig ([4fd37f2](https://github.com/qbit-ai/qbit/commit/4fd37f2bbd9af061b5d9c1f30ec9709477d3771c))
* **indexer:** add codebase management commands ([7f328ff](https://github.com/qbit-ai/qbit/commit/7f328ff6849eca7fcc487cbfb388907c26182872))
* **indexer:** add configurable global index storage location ([a647877](https://github.com/qbit-ai/qbit/commit/a647877550b80eb65dac2a0a1111af8b5044ecd3))
* **indexer:** add paths module for index directory resolution ([6988a0b](https://github.com/qbit-ai/qbit/commit/6988a0bf2885a8d347e785557c83664520f30806))
* **indexer:** integrate configurable storage location ([10d2bfb](https://github.com/qbit-ai/qbit/commit/10d2bfbbad175066ce13a4366721e18e1fdfb5ac))
* **pty:** detect alternate screen buffer via ANSI CSI sequences ([29e77b4](https://github.com/qbit-ai/qbit/commit/29e77b40478c69d5abc7866b9cfc21e711043883))
* **settings:** add CodebaseConfig schema for codebase management ([d742755](https://github.com/qbit-ai/qbit/commit/d74275588f0ae330dc716cb33ea9fc34ba83e091))
* **settings:** add fullterm_commands setting for custom TUI apps ([336c09f](https://github.com/qbit-ai/qbit/commit/336c09f52094231d042bc613b1cbd63af75e98ac))
* **settings:** add IndexLocation enum for configurable index storage ([2aa7b35](https://github.com/qbit-ai/qbit/commit/2aa7b354624dc69b2c8401d314f4c1c02971bd84))
* **shell:** add multi-shell support for bash and fish ([e629585](https://github.com/qbit-ai/qbit/commit/e629585828679e75cf69ce23c34f550c9b769370))
* **terminal:** add DEC 2026 synchronized output and improve TUI compatibility ([#48](https://github.com/qbit-ai/qbit/issues/48)) ([21b3cfd](https://github.com/qbit-ai/qbit/commit/21b3cfd743962d051bf07016e60ef2f0cd0cd550))
* **terminal:** add fullterm mode for interactive CLI apps ([016b1d7](https://github.com/qbit-ai/qbit/commit/016b1d724772e8d67f69c8fa8ebd8903e5796fa8))
* **ui:** add Codebases settings tab for managing indexed repositories ([81d30e0](https://github.com/qbit-ai/qbit/commit/81d30e0c4b775737222cb3e2b039a8c01143c138))
* **ui:** add sub-agent tool call details display ([92121f5](https://github.com/qbit-ai/qbit/commit/92121f5e1a7e4e9f1d62472ad27c2c94c612a559))


### Bug Fixes

* allow dead_code for unused HunkApplyError variant ([4e6d39d](https://github.com/qbit-ai/qbit/commit/4e6d39d2b48096aec10ac41d335ebc7c5d57c2d2))
* displaying shell and ai responses ([04d5f62](https://github.com/qbit-ai/qbit/commit/04d5f62dc495a16885c1176f7db0b9192d405841))
* resolve test failures after sub-agent merge ([7efb6ee](https://github.com/qbit-ai/qbit/commit/7efb6ee0e1e177be79d9d5cb0177499c8bab478b))
* resolve test failures and improve test stability ([84110d3](https://github.com/qbit-ai/qbit/commit/84110d369ee61f5e070cada9c01385d80c2a9808))
* **rig-zai:** add budget_tokens and debug logging for thinking mode ([9a0fac2](https://github.com/qbit-ai/qbit/commit/9a0fac25e05d271d2c416bd9bba35ac86a8c4c66))
* **tools:** improve error messages for file path resolution ([e45d2c2](https://github.com/qbit-ai/qbit/commit/e45d2c245a9ac2dbb5e87acd56e9f720c35da393))


### Refactoring

* **ai:** remove unused is_default method from AgentMode ([5271b1e](https://github.com/qbit-ai/qbit/commit/5271b1edb1d1bd99cdaf4f77197dfcf054704e93))
* **cli:** remove indexer initialization from CLI bootstrap ([ac55ca8](https://github.com/qbit-ai/qbit/commit/ac55ca8c08f0ded59afa7b32abc49b6411bac68f))
* rename project directories for clarity (src-tauri→backend, src→frontend) ([97c01e9](https://github.com/qbit-ai/qbit/commit/97c01e9ca886e65fbaccc1dd00b24b694680ed54))
* rename src-tauri to backend and src to frontend ([83ed990](https://github.com/qbit-ai/qbit/commit/83ed990ae410b4dbdab1926365fef62132f65b89))
