# dre Changelog

<!-- insertion marker -->

## [v0.3.1](https://github.com/dfinity/dre/compare/v0.3.0...v0.3.1) (2024-3-21)

### feat

* **cli:** Check that there are two download URLs for elect proposals (#236) ([1ac8581](https://github.com/dfinity/dre/commit/1ac8581e1d091170f66d7dff084d5479ca4e67db))
* **rollout:** Rollout controller refactoring part 1 - fetcher logic (#237) ([bf0b89a](https://github.com/dfinity/dre/commit/bf0b89a137348a705dff9ca21768513b62f89953))
* rollout-controller: Implementing action taking and refactoring tests (#249) ([6e6a377](https://github.com/dfinity/dre/commit/6e6a3772f766bf0c52e01151fd47fed733f27fff))

### chore

* Update dependencies (#225) ([820266c](https://github.com/dfinity/dre/commit/820266c888a1a1af3ffe2ac11248859033b433cd))
* Update dependencies (#240) ([996a004](https://github.com/dfinity/dre/commit/996a0043d26b99d8c664c78e1bfc769330a6d11a))
* Update dependencies (#244) ([4a6db31](https://github.com/dfinity/dre/commit/4a6db31796536f24a0e49a07faf3e2dc7fa812af))

### fix

* **dre:** DRE-147 Do not require neuron id for fetching trustworthy metrics (#251) ([03dfe1f](https://github.com/dfinity/dre/commit/03dfe1f1e9fc4780880d1957594a344f83382dd6))

## [v0.3.0](https://github.com/dfinity/dre/compare/v0.2.1...v0.3.0) (2024-3-6)

### docs

* Replace the version in the download URL with the "latest" (#137) ([765a35b](https://github.com/dfinity/dre/commit/765a35bafc21a94892c03781fb6b487cac727d5d))
* Update the Jupyter runbook and data for Trustworthy metrics (#138) ([af1ef6d](https://github.com/dfinity/dre/commit/af1ef6d84456836c08e04d224868a2d8733df479))

### fix

* **ci:** Use a GitHub PAT when creating PR automatically (#144) ([e1d36bc](https://github.com/dfinity/dre/commit/e1d36bc77e9f3013a8865a73973482b3d277a1b1))
* **sns-downloader:** fixing limit and better hashing (#190) ([8584071](https://github.com/dfinity/dre/commit/8584071742c20520f271e3f9974c18092c1c3c27))
* **log-fetcher:** binary fields that end with `\n` are not parsed correctly (#213) ([2d96e38](https://github.com/dfinity/dre/commit/2d96e384e66da55d64b6ad3ffc8dd036355fb29a))
* **ci:** Fix case of github_token variable (#217) ([9940fdf](https://github.com/dfinity/dre/commit/9940fdf8c85766fdc64964e7b932da14426f2d08))
* **downloader:** multiservice discovery should filter boundary nodes based on target name (#219) ([5251ab9](https://github.com/dfinity/dre/commit/5251ab9519d35d4d38bd8f01e6ca30db9d4728c1))

### feat

* **dre:** implementing a protection method for updating unassigned nodes (#151) ([c4205d2](https://github.com/dfinity/dre/commit/c4205d23e4eb12ce720b3c206f64fad098f2605c))
* **ci:** Bump up the actions/cache to v4, and force overwriting the cache (#164) ([f204cdc](https://github.com/dfinity/dre/commit/f204cdc3fd6992455c74d4e84f16309f0d10be9f))
* **registry-dump:** Interpret vec<u8> data in registry structs as corresponding types (#175) ([3be2000](https://github.com/dfinity/dre/commit/3be20009088f7d376a5a3bb8ed242175adcdb801))
* **release-notes:** Show start and end commit in the release notes (#186) ([4adbc7d](https://github.com/dfinity/dre/commit/4adbc7dd7e360b788fa98030bd1cdb746fc144e5))
* **msd:** metrics (#171) ([a480f59](https://github.com/dfinity/dre/commit/a480f59ebc80d34245e51cb465afd7f1499a4664))
* **release:** Proposing release index shema (#211) ([31e9076](https://github.com/dfinity/dre/commit/31e9076fb99dfc36eb27fb3a2edc68885e6163ac))
* **dre:** Migrated firewall rules from release repo to `dre` tool (#221) ([72ec3b3](https://github.com/dfinity/dre/commit/72ec3b3746e47a24d7bd2b3034141588c2570786))
* **rc:** started work on `rollout-controller` (#224) ([c6933c1](https://github.com/dfinity/dre/commit/c6933c10e35af82b49c75176d99e7955c27834d8))

### chore

* **deps:** bump ipython from 8.20.0 to 8.21.0 (#157) ([2778165](https://github.com/dfinity/dre/commit/27781656d450bb695553f685693881e8bcd465c0))
* **deps:** bump ansible from 9.1.0 to 9.2.0 (#156) ([d9d5eef](https://github.com/dfinity/dre/commit/d9d5eef771500ca6031a09a133dc6013f66a8e91))
* Update dependencies (#160) ([edbbdc0](https://github.com/dfinity/dre/commit/edbbdc0a0972d4aaee2c50f56a695d0a7e126edf))
* Update dependencies (#168) ([308f707](https://github.com/dfinity/dre/commit/308f707501b513cd772a61eae270db21713361fc))
* **pre-commit:** remove the legacy filter of the ic submodule (#177) ([350bd58](https://github.com/dfinity/dre/commit/350bd587a682ce0b81d19a78e25582715fcc0a62))
* **dashboard:** Update the version of prometheus-http-client (#178) ([989ca8d](https://github.com/dfinity/dre/commit/989ca8df70c64b21c54455768cbcf5f19e837b0f))
* **deps:** bump the npm_and_yarn group across 1 directories with 1 update (#163) ([bb86e0d](https://github.com/dfinity/dre/commit/bb86e0d18c36c86d5891edde2c64fbeb77a6ad45))
* **deps:** bump pre-commit from 3.6.0 to 3.6.1 (#184) ([6f8eec2](https://github.com/dfinity/dre/commit/6f8eec21b8ae39e0cf574b2f1e8693896c41ca0a))
* **deps:** bump mkdocs-material from 9.5.7 to 9.5.9 (#183) ([761a6ac](https://github.com/dfinity/dre/commit/761a6acf9da743ac296d42b038a37c4f4ec732fb))
* **deps:** bump atlassian-python-api from 3.41.9 to 3.41.10 (#182) ([9aa3dfd](https://github.com/dfinity/dre/commit/9aa3dfd97ff9aaafefdaef7b6b1ec55e60c32f80))
* **deps:** bump the cargo-dependencies group with 8 updates (#185) ([20d654e](https://github.com/dfinity/dre/commit/20d654e203c45ec0ac95638d25311e93af3e5937))
* **msd:** Improvements and cleanup (#188) ([9425eba](https://github.com/dfinity/dre/commit/9425ebadf3e52ac386239d5dc36b054e709c6638))
* **deps:** bump mkdocs-material from 9.5.9 to 9.5.11 (#210) ([d2ca785](https://github.com/dfinity/dre/commit/d2ca78562ca342c68b9c6aa8f8dbbfafc7954be9))
* Update dependencies (#199) ([a8ee432](https://github.com/dfinity/dre/commit/a8ee43224becfdb4c697ab9433f05ef0c89c68be))
* **deps-dev:** bump black from 24.1.1 to 24.2.0 (#201) ([0411fd7](https://github.com/dfinity/dre/commit/0411fd7ba2648da20643181243df5595137c3068))
* Update dependencies (#223) ([8d45d43](https://github.com/dfinity/dre/commit/8d45d43ef320d94570a7d5b7327d88327091593d))
* **deps:** bump poetry from 1.8.1 to 1.8.2 (#226) ([ed7142b](https://github.com/dfinity/dre/commit/ed7142bd0585940f97ff3c373d4951a14ec0687a))

### refactor

* **MSD:** migrate warp to axum (#162) ([096ed24](https://github.com/dfinity/dre/commit/096ed24c76d2ba92d6b05ee9d2c22539e103f6ba))

## [v0.2.1](https://github.com/dfinity/dre/compare/v0.2.0...v0.2.1) (2024-1-30)

### fix

* **k8s:** adding timeout option (#79) ([5842d80](https://github.com/dfinity/dre/commit/5842d80350d6b46cb67ca68f2ca72b2248bb2ac2))
* **k8s:** migrating to using gitlab api (#87) ([5ebed78](https://github.com/dfinity/dre/commit/5ebed7824bf4d2d6c577cdce19978a37f18f0ed1))
* **ci:** Run CI actions on PR and merge queues as well (#94) ([e11a3a2](https://github.com/dfinity/dre/commit/e11a3a2b9b48cbc065b3dc88f9f54f1cc0cb6ef2))
* **docs:** Update the links for running the trustworthy metrics notebook online (#92) ([c676c03](https://github.com/dfinity/dre/commit/c676c03804f16ce3177a3fe8cd160a54fe22ea07))
* **dashboard:** Automatically update the internal dashboard k8s deployment (#96) ([d6db5ee](https://github.com/dfinity/dre/commit/d6db5ee9108e520bdaca90bf2d7e9cda9a8d3f9c))
* **ci:** Update the reference to the gitlab api token (#98) ([fb5c3dc](https://github.com/dfinity/dre/commit/fb5c3dcc4afd70b0818f64a6a9291cf5c96c8248))
* **ci:** Cloning k8s repo to the correct directory (#103) ([0eab50d](https://github.com/dfinity/dre/commit/0eab50d06859b12bd1dfb4fad3982ae57dd5002e))
* **dashboard:** Use Debian Slim base image for OCI images by default (#105) ([a93c44b](https://github.com/dfinity/dre/commit/a93c44b14437c0792031a98367d0f7923d8ee2c6))
* **dashboard:** Use OCI images that have ca-certs and new glibc (#108) ([0ec6e76](https://github.com/dfinity/dre/commit/0ec6e760c0210ad7ac5418568b68d15a5ee7af70))
* **dashboard:** go back to the bitnami git image as distroless has no git :/ (#112) ([f76f58c](https://github.com/dfinity/dre/commit/f76f58cb1f1ce92e82ea31e30131044e346013f6))
* **ci:** Correctly pass secrets to the k8s-deployments composite actions (#120) ([41f0ce3](https://github.com/dfinity/dre/commit/41f0ce3bd0c8039691087c76fa713c8789edd764))
* **bazel:** Update build configuration and version information (#121) ([c85c81f](https://github.com/dfinity/dre/commit/c85c81f4676fce9ef72dccb6bba06fc898eccd33))
* **ci:** Prepare releases as a GH composite action instead of a workflow (#134) ([d1fe2c7](https://github.com/dfinity/dre/commit/d1fe2c7b4bbdefe772337b882d5c02d4b0772cf0))

### feat

* **ci:** nightly job to completely clean up bazel caches ([9d8593d](https://github.com/dfinity/dre/commit/9d8593d3ba49034407c952a2e0ac16c139bef8e8))
* **ci:** nightly job to completely clean up bazel caches ([258e3b8](https://github.com/dfinity/dre/commit/258e3b81d98acbacba85223a8fe5d75d435be298))
* **ci:** Enable grouping of dependabot PRs (#101) ([997d01f](https://github.com/dfinity/dre/commit/997d01f84b79e1845a51688eb0be29ab3de9729b))
* **ci:** Nightly update dependencies and cache (#102) ([54491c6](https://github.com/dfinity/dre/commit/54491c62e668d7fae1457cef16d46c2b73646172))
* **ci:** Create a PR on nightly CI runs instead of pushing to main directly (#114) ([53c2def](https://github.com/dfinity/dre/commit/53c2defcdb5627be2ad39e1ae0b6e905ae0df97a))
* **release-notes:** Migrate the release notes script to the DRE repo (#119) ([19c0c08](https://github.com/dfinity/dre/commit/19c0c08931bc3b202bfed1785b43f4c961207292))

### chore

* **ci:** Remove a duplicate step from the bazel workflow (#109) ([9dd26cb](https://github.com/dfinity/dre/commit/9dd26cbd128d63cd08cb382201aebe39c986ae51))
* **deps:** bump ansible from 8.7.0 to 9.1.0 (#84) ([71e0f91](https://github.com/dfinity/dre/commit/71e0f9172636006bc9290cde51f4e769f85727cc))
* **deps-dev:** bump black from 23.12.1 to 24.1.1 (#124) ([3c8c45c](https://github.com/dfinity/dre/commit/3c8c45cbd9b24018447e80d1a21f5b0f7ee1317b))
* **deps:** bump clickhouse-connect from 0.6.23 to 0.7.0 (#125) ([617335e](https://github.com/dfinity/dre/commit/617335e715c2270a82a9042555301ea498bf8f56))
* **deps:** bump mkdocs-material from 9.5.4 to 9.5.6 (#123) ([53bed11](https://github.com/dfinity/dre/commit/53bed116d3583dca9205b53f4aaa88ab25794dbb))

### docs

* Add some tips and tricks for our k8s ops (#115) ([fd165ed](https://github.com/dfinity/dre/commit/fd165eddcbc2d021a9ca2f348eb351033b4353ac))
* Update the readme and some more docs (#117) ([3f78296](https://github.com/dfinity/dre/commit/3f78296102a8b2cc7c2cbca226997d376a98e7fa))

## [v0.2.0](https://github.com/dfinity/dre/compare/v0.1.0...v0.2.0) (2024-1-19)

### fix

* mark jq as executable after downloading ([575d214](https://github.com/dfinity/dre/commit/575d214bd335d92380e5d49d7eb65ed326a67420))
* cd to the ic repo when checking the newest revision ([02d39bb](https://github.com/dfinity/dre/commit/02d39bb81f1f705a09d08285ae55529cb5a3d9c3))
* accept non-blessed binaries by default ([a2f210e](https://github.com/dfinity/dre/commit/a2f210e1d9b561514556312de22e0cd73a7282af))
* **Decentralization:** 1/3 limit is a strict 1/3, not 1/3-1 ([51eb5e5](https://github.com/dfinity/dre/commit/51eb5e538af85a94f3a8bafb946e00df517a6a38))
* **backend:** Make registry polling less frequent to improve robustness ([d86a0a2](https://github.com/dfinity/dre/commit/d86a0a274c3cbe713080ab878298efa7a60c9aea))
* Increase timeout for the qualification test ([8265432](https://github.com/dfinity/dre/commit/8265432d1bd300649a1eec2d5d4ad429fbbe74f9))
* **cli:** Handle missing commit hash in the release notes ([d151a74](https://github.com/dfinity/dre/commit/d151a742ecc582819d5d4f2b73564d634721cc54))
* Minor adjustments in the IC-OS verification instructions ([7ac6a08](https://github.com/dfinity/dre/commit/7ac6a0845398460861baeaad5711662d1c81ba43))
* Dashboard releases only from master ([b234fae](https://github.com/dfinity/dre/commit/b234fae5639bcd0e7ffbd4ee24c6d84e070a8c36))
* Release qualification should not wait block on dashboard publish ([38f27ac](https://github.com/dfinity/dre/commit/38f27ac80a540caebe051ece91d482a3dbe26cdd))
* Build verification instructions should work with existing old image ([aacbcad](https://github.com/dfinity/dre/commit/aacbcada259f1cb1df8de33f2bb9206a593d411b))
* **cli:** Do not inject dry-run and confirmation if --help provided ([85fc786](https://github.com/dfinity/dre/commit/85fc786f60f3715787aa586f8f710d4afb8246ab))
* **cli:** Do not supply auth args to ic-admin get commands ([c4a1705](https://github.com/dfinity/dre/commit/c4a17056cc0f65c16d45aac9c16d33e4b600d4e1))
* **cli:** Fix auth for proposals to staging ([937a643](https://github.com/dfinity/dre/commit/937a643c30cb37f158284a9fab283c6768c75f4a))
* **cli:** Fix instructions for verifying sha256 of the update image ([68cd941](https://github.com/dfinity/dre/commit/68cd941e2385078f06ecd0d5e393698ea177eccb))
* **dashboard:** Correctly show nodes as decentralized even if they are not in FactsDB ([b83be5a](https://github.com/dfinity/dre/commit/b83be5accbcdc25a50ca58ee442824a0c65e3fa0))
* **dashboard:** Fix //rs code path references ([f6d4a11](https://github.com/dfinity/dre/commit/f6d4a110150bb00137dc00c57b62cac2384bcd6a))
* **dashboard:** Sort subnets deterministically in the release rollout dashboard ([9b71f7b](https://github.com/dfinity/dre/commit/9b71f7b8c5b7a1521c66033a1d52541a1192d76b))
* **cli:** Use zip_longest when showing a diff of node changes ([db04589](https://github.com/dfinity/dre/commit/db04589cdb99fd22911914f858517edce2ed3cca))
* **backend:** Retireable versions should not include actively used versions ([2cf0f98](https://github.com/dfinity/dre/commit/2cf0f9880d7e84b5b46a8d07c56b200067a691c5))
* **backend:** Make more operations deterministic ([c4f7745](https://github.com/dfinity/dre/commit/c4f7745cb25c7261fbfc25d5f185e956f473054f))
* Relax the p8s alert query, to include tickets and info alerts ([f194640](https://github.com/dfinity/dre/commit/f194640046741bd8cf7a05b157fdeb3c8e4046b0))
* **cli:** Ensure ic-admin version string does not have a newline ([1d8291c](https://github.com/dfinity/dre/commit/1d8291cd71e4cca7ab606b6c3a870ac31293913f))
* **cli:** Make --simulate a global arg and enable it for all subcommands ([91aaf0b](https://github.com/dfinity/dre/commit/91aaf0b43bcf70b0b6be74ccb4b6c7b7a6f3a99e))
* **ci:** Variable name ([a6b76be](https://github.com/dfinity/dre/commit/a6b76be39daaa82ba1507388735c3224423c9a53))
* **backend:** Only show node as degraded if ALERT is firing ([2c5de64](https://github.com/dfinity/dre/commit/2c5de641ebcb35b58f067c86ddeaa9af9acfb3db))
* **backend:** Show the correct number of nodes replaced instead of requested ([d91e613](https://github.com/dfinity/dre/commit/d91e61386c8707c09b9ac71cf44b5f5fd1846033))
* **dashboard:** Fix subnet id showing and searching ([154478e](https://github.com/dfinity/dre/commit/154478e2628db05275b94a7ee67d9377c4a25efb))
* **dashboard:** tsc was complaining about types ([60d587b](https://github.com/dfinity/dre/commit/60d587bede8951a6d7b32089b0b1c33de72e34fd))
* **README:** git hook install ([8bf7611](https://github.com/dfinity/dre/commit/8bf7611743163e85390a0ab53807a80d9728a5e3))
* **cli:** Verify the download URLs before submitting a proposal ([0e71882](https://github.com/dfinity/dre/commit/0e71882ae9af095118c3c6c484073e2ccc642324))
* **cli:** Version unelect code fixed and cleaned up ([17c839a](https://github.com/dfinity/dre/commit/17c839ab7cf4ab6d56feedbc153531183ef03514))
* **qualification:** Brought back to life qualification on staging and factsdb ([29f654a](https://github.com/dfinity/dre/commit/29f654a31d9a79683a47966429a54b678f1d0349))
* **backend:** Unconditionally make a full clone of the IC repo ([c2d1a31](https://github.com/dfinity/dre/commit/c2d1a3112c48228691a89526c51b62d969b0f86b))
* **scripts:** The IC CDN does not have openssl-static-binaries anymore ([42e6e60](https://github.com/dfinity/dre/commit/42e6e60f986263cc774362618e59c1c9a3fbc115))
* **backend:** Only remove dead nodes from the registry by default ([938d675](https://github.com/dfinity/dre/commit/938d675ee3eb538575a20e92231fcf0bcecc865c))
* **bazel //:**mkdocs ([294730a](https://github.com/dfinity/dre/commit/294730a27f8408c81866cc67d94c0edb6cca881d))
* **ci:** Use git repo cache in /tmp ([51677d2](https://github.com/dfinity/dre/commit/51677d2676dccd4c642d2742de52d62c1592fbb1))
* **ci:** git test for the ic repo does not work within bazel ([fb5fdd6](https://github.com/dfinity/dre/commit/fb5fdd6d32f2f69db8d0a16a3679f76b6501a572))
* **ci:** revert unnecessary change ([bd51781](https://github.com/dfinity/dre/commit/bd517818baa630b862c94aec80cd451845161fcd))
* **ci:** Use older Ubuntu image to use older GLIBC (#49) ([24e8cd9](https://github.com/dfinity/dre/commit/24e8cd90b885f7f730baa765f1c16c7d6c513c60))
* multiservice-discovery: registry sync returns on missing nns_public_key and msd pings nns before adding the definition (#48) ([deb0dfa](https://github.com/dfinity/dre/commit/deb0dfa3521034cf72079fec92eae173a510fa85))
* **ci:** Set the DRE cli version based on the tag or git rev ([a741f80](https://github.com/dfinity/dre/commit/a741f802476f0685f3b6e79475b78c0d61eb0e72))
* **k8s:** setting larger python image for cursor-initializator-elastic (#75) ([cd5d96d](https://github.com/dfinity/dre/commit/cd5d96d0be1b17d4557399d9cac9e9e2449997c4))

### chore

* Update the staging bootstrap for the latest IC version ([f9c8b65](https://github.com/dfinity/dre/commit/f9c8b651ed6c76c9a8533f3f6d728d0cd56b8cfb))
* bump up a few dependency versions ([1d3e6cd](https://github.com/dfinity/dre/commit/1d3e6cd9b7b27ecabe3081fca0824a21f3eef30f))
* **BOUN-525:** setup config for certificate issuance ([251c0c8](https://github.com/dfinity/dre/commit/251c0c823154eca6694378a86b1f798370a89386))
* update neuron id for Ege -> Maksims ([5cd9ec4](https://github.com/dfinity/dre/commit/5cd9ec4c5c07180deaba381d9b996b065ef850a4))
* Update neuron 60 mapping to Andre Popovitch ([4778970](https://github.com/dfinity/dre/commit/47789708a2a25927cdca27638ecde60867da42b3))
* Update the default ic-admin version used by the release_cli ([3cc4859](https://github.com/dfinity/dre/commit/3cc48594f6979e9b7a60a99b441214fe02107c1e))
* Stop sending notifications to #eng-testing for ic-versions update ([40b8e3a](https://github.com/dfinity/dre/commit/40b8e3af1b7e57cd1096f3d2411293116f72ad80))
* Minor refactor in the IC version retire, no functional change ([7177fa8](https://github.com/dfinity/dre/commit/7177fa8a12bb568ca7624a558c68c42b925bdba2))
* Hide neuron 20 in the NNS proposal notifications in slack ([c53e03e](https://github.com/dfinity/dre/commit/c53e03e6114fca6a4299ced608066cd450ab64e8))
* Update the neuron slack mapping ([4051f9b](https://github.com/dfinity/dre/commit/4051f9bfbb6d55ab35bfa5a0bdbe87bd6eef7fc9))
* Another minor adjustment of the bless proposal summary ([c1dc347](https://github.com/dfinity/dre/commit/c1dc3472af615e7fe673646c29a8ef97c2b3c869))
* Do not mention @release-engs on automation proposals ([116ba49](https://github.com/dfinity/dre/commit/116ba499c150c19a7fada40ee2501bfafd3d9a72))
* Update IC dependency and rustc version ([093f701](https://github.com/dfinity/dre/commit/093f701e33642f4c5867a81129017207ca1aa829))
* Update the submodule update in the README.md ([2bfbb09](https://github.com/dfinity/dre/commit/2bfbb098bf3aa673b40ca754de6de71031e843b3))
* Further adjust/restrict when some CI jobs run ([6caee38](https://github.com/dfinity/dre/commit/6caee388df5826072e4e49eda4ed53082fcea111))
* **slack:** Update the slack-neuron mappings ([fc25887](https://github.com/dfinity/dre/commit/fc25887f01a79cff6396d9ec0ef441a6abdaca1d))
* Bump up a few more dependencies ([aa875ad](https://github.com/dfinity/dre/commit/aa875ad1c980726bd9e60392d611ebb4e4ffaf28))
* **cli:** Update the instructions for verifying the IC image sha256 ([630756c](https://github.com/dfinity/dre/commit/630756c8a5cd8ca60f42657a02b9218e2d5c4d8a))
* **cli:** Fix retire version message ([6fe9998](https://github.com/dfinity/dre/commit/6fe9998a5dcf368828f17c8ee60fa4a4e6f5de95))
* Bump up the IC dependency version ([8528ba0](https://github.com/dfinity/dre/commit/8528ba000ed2518b41705b5ca2a819476d6d9c92))
* **decentralization:** Count and try to reduce the number of nodes of dominant actors in a subnet ([54fedb8](https://github.com/dfinity/dre/commit/54fedb8f4e1ced61ef352c08c256e461a3f4084c))
* **cli:** Show the used ic-admin version, for clarity ([16e9629](https://github.com/dfinity/dre/commit/16e9629413eb140924fad85be3657d7781656338))
* **slack:** Update neuron 42 mapping from Raghav to David ([9729f2a](https://github.com/dfinity/dre/commit/9729f2a4b12edfd38f635fbcabb99fb498817e02))
* **slack:** Update neuron mapping Yvonne-Anne -> Maksym ([8a6d4af](https://github.com/dfinity/dre/commit/8a6d4af8513dbd8f0b7943f467292f473bfe7ad1))
* **decentralization:** Minor optimization in the calc of the Nakamoto Coefficients ([29cbcb4](https://github.com/dfinity/dre/commit/29cbcb4d4ede12c8faeb9b698cc9483b60fba88b))
* **decentralization:** Remove unused dependency ([1af2cf4](https://github.com/dfinity/dre/commit/1af2cf406fb9dc9ced6e13f15a2fdeb9c181ba1c))
* **dashboard:** A few more subnet labels ([39509d5](https://github.com/dfinity/dre/commit/39509d52e802c53a3290e9b63e72768c91640a8e))
* **cli:** Use short git versions in the elect version proposal title ([56ce8a8](https://github.com/dfinity/dre/commit/56ce8a889b7946224c9795d61057cb7ef1ebdfda))
* **dashboard:** Remove non-working dialog to Create Subnet ([35c45cc](https://github.com/dfinity/dre/commit/35c45cc87c908cc93b7246fc600ebf0f900f26d9))
* **backend:** Remove unnecessary serde_as usage ([1a7e81b](https://github.com/dfinity/dre/commit/1a7e81b74983ff946eaa37dee1e344419d242471))
* **cli:** Remove the separate subcommand to retire replica versions ([17aad3b](https://github.com/dfinity/dre/commit/17aad3b8ace1f0b2d529005419ee6c41b645caad))
* **dashboard:** Bump multiple dependency versions (IC and crates) ([4e474a7](https://github.com/dfinity/dre/commit/4e474a7ac6fe2ad390a68ce5f95279b815c005bd))
* **backend:** Separate and simplify the function for displaying proposed subnet changes ([73109b3](https://github.com/dfinity/dre/commit/73109b3027166411e240acea9045943713083040))
* **backend:** Refactor backend and some other code, no functional change ([9efc6e4](https://github.com/dfinity/dre/commit/9efc6e4dc9632553076d43748e262225af047bc1))
* **qualification:** Remove unused elasticsearch data ingestion from qualification ([a85c5eb](https://github.com/dfinity/dre/commit/a85c5ebe5e19ab232bccf1924ec6dee62f718d47))
* **staging:** Updating scripts for bootstrapping staging ([cd34e17](https://github.com/dfinity/dre/commit/cd34e176f886794f6f7718af3e0ef7e30947fcd8))
* Allow Python >3.8 and <4 in the repo ([dc7e35f](https://github.com/dfinity/dre/commit/dc7e35f223de837ac6f0ffc8c9b253bb02470f6d))
* Update neuron id -> slack id mappings ([310cfed](https://github.com/dfinity/dre/commit/310cfed35d887a2d985c5c4589676a84ccb5be47))
* **factsdb:** Publish factsdb results as public gitlab snippets ([3f9123c](https://github.com/dfinity/dre/commit/3f9123c5f51af43ecbed5605d8f41cbaf30824f6))
* **audit:** Remove tracing-futures ([4709c6f](https://github.com/dfinity/dre/commit/4709c6fc4bd975f89d630a1ed431e8d662dbb0e8))
* **cli:** Minor improvements in argument parsing ([74c85ef](https://github.com/dfinity/dre/commit/74c85ef542928d4e612063e15280c54ca38d07fe))
* **cli:** Improve voting robustness ([583280b](https://github.com/dfinity/dre/commit/583280b1730e17f7c41f267da35ec5d66948a9b9))
* Update neurons-slack-mapping.yaml to remove Marin and Luis ([a15adb9](https://github.com/dfinity/dre/commit/a15adb9d9e2ac9fd2ed0bf983d8131e9d7f4e828))
* Update the Python to 3.11 and pre-commit config ([9d332f5](https://github.com/dfinity/dre/commit/9d332f51c4572e464f5e3417b387db088b57302d))
* **backend:** Remove erroneous prinln ([7f64667](https://github.com/dfinity/dre/commit/7f64667a7a4d147a7f689ea82a4071756c607442))
* Update the Python to 3.11 and pre-commit config ([b2b4603](https://github.com/dfinity/dre/commit/b2b46039ac90fdbbdf252a14eb590f253236d85a))
* **canisters:** Move canisters from //rs to separate dir (#44) ([b22c357](https://github.com/dfinity/dre/commit/b22c3578855d06cf9d8e7b1926181a25507f15f1))
* **deps:** bump the pip group across 1 directories with 2 updates (#55) ([b97ce2e](https://github.com/dfinity/dre/commit/b97ce2e247556e685b5d4b5ec1a35ab7b25b37b2))
* **deps:** bump the npm_and_yarn group across 2 directories with 1 update (#57) ([75b6adb](https://github.com/dfinity/dre/commit/75b6adbaf48f02d3b73b5b638eab79622ab881db))
* Consolidate and deduplicate cargo deps (#64) ([31bb6df](https://github.com/dfinity/dre/commit/31bb6dfa65fec246835a722a7ace80fed34a1f48))
* Bump multiple versions and prepare for a new release (#78) ([c112389](https://github.com/dfinity/dre/commit/c112389f146b9cf13e101feaa5e1785e4ad25683))

### feat

* Accept min Nakamoto Coefficient for subnet create ([73d17d0](https://github.com/dfinity/dre/commit/73d17d0795058fb90d4748e096fb4799ed0a1989))
* Add an endpoint to query subnet decentralization ([43bf059](https://github.com/dfinity/dre/commit/43bf059932f43b348980cfdd4e051b915197a3df))
* **cli:** Add verbose option to print the node replacement search ([fd3089d](https://github.com/dfinity/dre/commit/fd3089d9e588d1c22527c29a47bd644c7bac8063))
* Use decentralized nodes for replacements ([5769778](https://github.com/dfinity/dre/commit/5769778774ee388cf5a7cc9ace04d986bb9c3fc9))
* **dashboard:** split lines in the subnet replace penalty explanations ([445b165](https://github.com/dfinity/dre/commit/445b1653d4ca7e4ea00dba734df14140d4e8e28e))
* **slack:** Add a link to the internal dashboard for the subnet membership changes ([b2166c3](https://github.com/dfinity/dre/commit/b2166c30fe24fbebb2352b96d970348594d6aba4))
* Random best Nakamoto score node choice ([0676b19](https://github.com/dfinity/dre/commit/0676b19e319436dc25195cfc0d4402fd39f2a3b7))
* **cli:** Adding ability to resize a subnet by adding or removing nodes ([cd097f6](https://github.com/dfinity/dre/commit/cd097f6f1c8c1f38492ecd49d775e168467168d8))
* **cli:** Add support for excluding features or nodes when removing ([36d0715](https://github.com/dfinity/dre/commit/36d071583b022be3d64a2e1dd0504736d381488c))
* Increase verbosity if parsing refs fails, to aid debugging ([5196692](https://github.com/dfinity/dre/commit/519669208b3de02b0baea5860b2e195795961d54))
* **backend:** Support running with GITLAB_API_TOKEN env var ([8ca3b25](https://github.com/dfinity/dre/commit/8ca3b25c45c595715598b378a2c97d71c7475e41))
* **backend:** Improve the Nakamoto comparison ([9d7149a](https://github.com/dfinity/dre/commit/9d7149a319f301c02089796809b6695170354c3f))
* **dashboard:** CLI instructions for healing a subnet ([6e1574a](https://github.com/dfinity/dre/commit/6e1574a43854e5304351e458885f14fc4c169996))
* a git hook to update ic submodule on git pull ([b0e0c99](https://github.com/dfinity/dre/commit/b0e0c99de5a0a5c6fc73a4314e7bfbf875030f40))
* **cli:** Add github URL to the proposals for new IC version ([4ab357a](https://github.com/dfinity/dre/commit/4ab357aebe0f155c69355d6929327e8cf64a50bb))
* **cli:** Support for a stand-alone CLI binary, part 1 ([ed4cbc1](https://github.com/dfinity/dre/commit/ed4cbc11cfd9a03e2d7ae8c89ac4f8541f5054cf))
* **backend:** Clone public IC repo instead of depending on private gitlab ([e490a1d](https://github.com/dfinity/dre/commit/e490a1d94a03bf8c995cea34a2525a9c2bfcb305))
* **dashboard:** Support setting the ic repo cache dir from env ([f649f18](https://github.com/dfinity/dre/commit/f649f18a17e5e3b7bb638ebaf467e91b5f5c6709))
* **cli:** Include R2 download links into the elect proposals ([d435d59](https://github.com/dfinity/dre/commit/d435d59a3ee979224cc3479103d2140efcfbcbb3))
* **cli:** Replace unhealthy nodes based on ALERTS ([8febdea](https://github.com/dfinity/dre/commit/8febdea4f7f35f940cf6b9f1c05b6f6e94822393))
* **staging:** Make staging deployment less dependent on factsdb ([1afebd8](https://github.com/dfinity/dre/commit/1afebd8d2d52d7d0b37647c3061f951479d2f967))
* **cli:** Strip md/html comments from release notes ([1c809d6](https://github.com/dfinity/dre/commit/1c809d604233577e78557686599397773acb5513))
* **backend:** Add special handling for the European subnet ([036c0ba](https://github.com/dfinity/dre/commit/036c0ba80ce854a4b4860f247c48d8d8d57e842b))
* **docs:** Added mkdocs for documentation ([9d9f184](https://github.com/dfinity/dre/commit/9d9f1840b31eb695f90a37e5ca7dfc2a0de925c4))
* **observability:** migrating code from IC repo (#46) ([e9a363b](https://github.com/dfinity/dre/commit/e9a363b5efd9fb7be192a83694f44463f613b889))
* **cargo:** consolidate versions and project organization (#35) ([dbf0cc3](https://github.com/dfinity/dre/commit/dbf0cc355070fc6623975209b41962ff9860356b))
* **ci:** Make github releases on git tag push (#53) ([378dcf3](https://github.com/dfinity/dre/commit/378dcf3ba72d35ff0d9b44e610ee8df0815bfb89))
* **ci:** Automatically push containers if branch name starts with "container" (#65) ([97b98ef](https://github.com/dfinity/dre/commit/97b98efaffe791a64f0f646fed379731bcfe923a))
* **k8s:** Moving k8s python scripts to our repo (#70) ([e72e874](https://github.com/dfinity/dre/commit/e72e874c5305e87bae2c3a155426b67cb97171df))
* **k8s:** adding missing script for creating tables in clickhouse (#72) ([eed35d8](https://github.com/dfinity/dre/commit/eed35d8a340005dfeb9942ae784d4fd905946ad7))

### docs

* **mkdocs:** deploy mkdocs (#66) ([cdbe31f](https://github.com/dfinity/dre/commit/cdbe31f877c60e02afdc5a44e78e066990b52789))
* adding some content (#67) ([fe3941d](https://github.com/dfinity/dre/commit/fe3941d87c33fea15851a254754b64251e7148d9))
* Add docs for trustworthy node metrics (#71) ([7df6bb4](https://github.com/dfinity/dre/commit/7df6bb434d0a265b6341612750162666eb3e6a53))
* Update docs and dependencies for trustworthy metrics (#76) ([4b7d594](https://github.com/dfinity/dre/commit/4b7d5945d070dec62ab6b54c2923185b23cd3680))
* Update docs and dependencies for trustworthy metrics ([9b17936](https://github.com/dfinity/dre/commit/9b17936ef9d4f3284d30a38aa878b8615f5fcecb))
* Add architectural doc for trustworthy node metrics (#77) ([d20c679](https://github.com/dfinity/dre/commit/d20c679d6c80a942cbc767d651e077bfd73c957f))

::> 1000 commits in 5 version tags, last considered commit: e6229b71100b863829e292fa3e28957572572f26
