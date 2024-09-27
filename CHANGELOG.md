# dre Changelog

<!-- insertion marker -->
## [0.5.5](https://github.com/dfinity/dre/releases/tag/0.5.5) - 2024-09-09

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/d3c9efa02d88bab5454e795c50acb6d1575c5ecf...0.5.5)</small>

### Features

- Add a manual CI job that cleans bazel caches (#890) ([38b7bb9](https://github.com/dfinity/dre/commit/38b7bb959ff9caaa2f6bfe5decaf61c47ab0d8e2) by Saša Tomić).
- security fix proposal (#882) ([d4ce9d0](https://github.com/dfinity/dre/commit/d4ce9d013da8f2509a6614062bd3f17ab2375471) by Nikola Milosavljevic).

### Bug Fixes

- only require our label for bazel cache cleanup job (#891) ([2004230](https://github.com/dfinity/dre/commit/20042307d55785071bc1e57338acaa6190713e09) by Saša Tomić).
- --yes on propose (#892) ([66f2e5f](https://github.com/dfinity/dre/commit/66f2e5fb4eae2131ad68e3679c19f7997b845861) by Luka Skugor).
- initialization logic (#889) ([a1f42a7](https://github.com/dfinity/dre/commit/a1f42a757c63e5e2f1cfdf42a03ebd579fe86310) by Nikola Milosavljevic).
- add missing bins (#887) ([9f327c3](https://github.com/dfinity/dre/commit/9f327c378ac09be8882c76a0c98383ebe8ba535c) by Luka Skugor).
- fix the forum category code (#884) ([0351499](https://github.com/dfinity/dre/commit/035149972605d5245f61e122f2ec56600771fa64) by Luka Skugor).
- fix finding the forum category (#881) ([ce39a3e](https://github.com/dfinity/dre/commit/ce39a3e8baddc359a45f64e66e3718d25287c3d7) by Luka Skugor).

## [0.5.4](https://github.com/dfinity/dre/releases/tag/0.5.4) - 2024-09-05

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/e3b94efb3d36a89b7c981813626719ed48d5ec2f...0.5.4)</small>

### Features

- overriding ic admin versions (#864) ([69b4cf3](https://github.com/dfinity/dre/commit/69b4cf38e75c617374774199ff45487ddff7201d) by Nikola Milosavljevic).
- dre testing of `update-unassigned-nodes` command (#858) ([415f697](https://github.com/dfinity/dre/commit/415f6978960d701d7b7861542e6865160dc828f2) by Nikola Milosavljevic).
- Replace poetry with rye for managing Python dependencies (#857) ([00e419f](https://github.com/dfinity/dre/commit/00e419f85fb9dff6754447206047ab690cf044c0) by Saša Tomić).
- improve exclusion filters (#855) ([c5940e8](https://github.com/dfinity/dre/commit/c5940e8bc889a14065236af1f172ef71b49c96bb) by Luka Skugor).

### Bug Fixes

- Update cache job fix (#874) ([6f46268](https://github.com/dfinity/dre/commit/6f46268f44848c022ceb7fac4c35a543c60a17d4) by Saša Tomić).
- use container image with rye for update-dependencies (#871) ([f692898](https://github.com/dfinity/dre/commit/f692898a6ac1bcacfcb52310ab4b0e6b9bc358a8) by Saša Tomić).
- fixing default value for ic-admin-version (#872) ([9dd0e95](https://github.com/dfinity/dre/commit/9dd0e955f24b48bfb776e130f27ce419600a966e) by Nikola Milosavljevic).
- updating leftover runner images (#860) ([9e58a66](https://github.com/dfinity/dre/commit/9e58a66df73ce8d9f6bb896b6f6680836cfd5b44) by Nikola Milosavljevic).
- registry canister (#854) ([65e1de7](https://github.com/dfinity/dre/commit/65e1de743d99f0ee1db105f59ed9ebe0cb947474) by Nikola Milosavljevic).

## [0.5.3](https://github.com/dfinity/dre/releases/tag/0.5.3) - 2024-09-03

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/09221a6073f9b283fcfc5c16e9b430ff0fba1d87...0.5.3)</small>

### Features

- Add forum post link to commands for proposal discussion (#830) ([7c8f4be](https://github.com/dfinity/dre/commit/7c8f4be9a4e871d34f19eb4e17db217915fedd0b) by Saša Tomić).
- add the link to full changelog in the proposal (#820) ([248ef92](https://github.com/dfinity/dre/commit/248ef924819e4faef257d4ed1834e1b3a6a33ab1) by Luka Skugor).
- Provide more details on the node replacement proposals (#816) ([c161563](https://github.com/dfinity/dre/commit/c16156365f4718902e90e9f075f51fdbfaa48cc3) by Saša Tomić).
- Add distance from the target topology as business rules (#817) ([3752142](https://github.com/dfinity/dre/commit/37521426eefb31547f03e1dfea43ea8d213b710a) by Saša Tomić).

### Bug Fixes

- canister handshake error (#850) ([238e164](https://github.com/dfinity/dre/commit/238e164f88bbb19db4f09d5dc7b65f4e891bbf6e) by Nikola Milosavljevic).
- autodetection of HSM (#845) ([e666fd5](https://github.com/dfinity/dre/commit/e666fd530902ff22d0ca2b7f75de278791919584) by Nikola Milosavljevic).
- typo in the description of a NakamotoScore change (#832) ([931d1e0](https://github.com/dfinity/dre/commit/931d1e0f22f2774871dd3eb1661807e2b0c8a51d) by Saša Tomić).
- handle errors and retry (#824) ([936a17b](https://github.com/dfinity/dre/commit/936a17bb57a49c76f6bba8c74cae31a2794e4000) by Luka Skugor).
- wrong variable (#822) ([e74a315](https://github.com/dfinity/dre/commit/e74a315c4d3c4d618c15000ee10d9622297c9305) by Nikola Milosavljevic).

### Code Refactoring

- ic admin (#835) ([2a1657e](https://github.com/dfinity/dre/commit/2a1657e40510dc4cf98de5dae40ced720ca04d79) by Nikola Milosavljevic).
- lazy git (#837) ([60c745f](https://github.com/dfinity/dre/commit/60c745f6b66c1b692ce5b53491e2092cfda4756e) by Nikola Milosavljevic).
- lazy registry (#831) ([ceeb19b](https://github.com/dfinity/dre/commit/ceeb19b7a9667d311603dd0a1521c055b9111366) by Nikola Milosavljevic).

## [0.5.2](https://github.com/dfinity/dre/releases/tag/0.5.2) - 2024-08-27

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/e560ab4487484cedfd5162e03aa45824376d93ef...0.5.2)</small>

### Features

- Include node health when resizing or creating a subnet (#801) ([3c91b71](https://github.com/dfinity/dre/commit/3c91b71f293fc344703b859eb4e0f5d81b9df9d6) by Saša Tomić).
- automatic pr for successful (#793) ([4ce76af](https://github.com/dfinity/dre/commit/4ce76af036daf206afe21240464bf95a2ac3a263) by Nikola Milosavljevic).
- use .zst images instead of .gz (#797) ([d785043](https://github.com/dfinity/dre/commit/d78504372d07135e78a83e1dae0ad4cfe1f187fb) by Luka Skugor).
- Optimize subnet healing process (#780) ([37ccfff](https://github.com/dfinity/dre/commit/37ccfffe5d9281f1229c93e8ae5916d1bee8e985) by Saša Tomić).

### Bug Fixes

- `update-unassigned-nodes` logic in the wrong place (#805) ([3e1445d](https://github.com/dfinity/dre/commit/3e1445db102584f255a8a1e853a7db039dd2dfb9) by Nikola Milosavljevic).
- Update subnet analysis command to allow adding and removing multiple nodes at once (#802) ([ef80ac2](https://github.com/dfinity/dre/commit/ef80ac2978cad865dcb2fa02523f7fc86068e6dd) by Saša Tomić).
- Make node metrics retrieval parallel again, for improved performance (#804) ([867cb3d](https://github.com/dfinity/dre/commit/867cb3da95c12d278aa470c4811449738323fa9f) by Saša Tomić).
- adding back output of proposals (#798) ([9169836](https://github.com/dfinity/dre/commit/9169836097b9436b5949fe8e1616438c6d1722af) by Nikola Milosavljevic).
- Skip rc branches without date (#796) ([3d398e9](https://github.com/dfinity/dre/commit/3d398e9326940a4462d872f1f55dccec58f122ac) by Luka Skugor).
- dre heal should not consider unhealthy nodes as replacement candidates (#794) ([9e99906](https://github.com/dfinity/dre/commit/9e99906bd03326a2348e901905b0cd13af8a7d0c) by Saša Tomić).

## [0.5.1](https://github.com/dfinity/dre/releases/tag/0.5.1) - 2024-08-23

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/c2530ebf566627158e7a99759b2d95e4e5751118...0.5.1)</small>

### Features

- Add whatif command for analyzing decentralization (#767) ([31ab0a1](https://github.com/dfinity/dre/commit/31ab0a12c6ff40862f0c8a492ed666d1c924a64f) by Saša Tomić).
- slack notification about quali (#764) ([c32aa4f](https://github.com/dfinity/dre/commit/c32aa4f399497eec9770d871a05ff409b031d10c) by Nikola Milosavljevic).
- uploading of artifacts (#755) ([5b1474d](https://github.com/dfinity/dre/commit/5b1474d025d31b937a83d32b0f41ab8dfd25434a) by Nikola Milosavljevic).
- implementing multiple starting versions (#743) ([6b7626b](https://github.com/dfinity/dre/commit/6b7626b6409cad300ecc0965947f8e34b3331cd8) by Nikola Milosavljevic).
- various improvements (#742) ([26e6ce0](https://github.com/dfinity/dre/commit/26e6ce0e21ceabeb45176fa2b43971c6947af486) by Luka Skugor).
- top up neuron request (#752) ([4866612](https://github.com/dfinity/dre/commit/48666128ab051859bc240f002573a965d4fe37fe) by Nikola Milosavljevic).
- adding resolving version from nns via rollout dashboard (#740) ([a84a727](https://github.com/dfinity/dre/commit/a84a72754e3b8a39eade7bc162555264ad0099b1) by Nikola Milosavljevic).
- [DRE-240] Add more info to the subnet membership change proposals (#736) ([c980019](https://github.com/dfinity/dre/commit/c980019edb2354d6d606387294316e7d32e42541) by Saša Tomić).
- Add time to all dre tool logs, and remove it from voting events only (#726) ([bbf1ad5](https://github.com/dfinity/dre/commit/bbf1ad59700db7a0063a217840925cd4c9cb03d2) by Saša Tomić).

### Bug Fixes

- Improve numbering in create new neuron instructions (#776) ([e638cf8](https://github.com/dfinity/dre/commit/e638cf85d234025a3f1d4cd4dac6f1c3275882a6) by Saša Tomić).
- Fix the duplicate output (printout) of the SubnetChangeResponse (#762) ([b1130f0](https://github.com/dfinity/dre/commit/b1130f0630296008aabd94d1305465eeb7d41c2b) by Saša Tomić).
- continue on error for creating mr on k8s repo (#754) ([46f36b8](https://github.com/dfinity/dre/commit/46f36b8b153c8a6db2a3be6d3cb619c5c8468b74) by Nikola Milosavljevic).
- disabling devtools (#749) ([a948a92](https://github.com/dfinity/dre/commit/a948a928d72cc307e86e9f72b57f7042eb6bcc63) by Nikola Milosavljevic).
- updating refs in update-k8s-deployments (#748) ([2a1f3a7](https://github.com/dfinity/dre/commit/2a1f3a7aa42e5000de140188f5a01c0a85297c8a) by Nikola Milosavljevic).
- Fix the "proposals analyze", and improve the output (#746) ([c6c74dc](https://github.com/dfinity/dre/commit/c6c74dccc9e44254bac315fbdbd8a66da66a46fd) by Saša Tomić).
- [DRE-241] Prefer existing nodes in subnet when selecting best results (#734) ([ec22c7c](https://github.com/dfinity/dre/commit/ec22c7ca6d04f8480721b2a1e71466d088ab902f) by Saša Tomić).
- schema root namings (#735) ([f8935f6](https://github.com/dfinity/dre/commit/f8935f6a12b9e0fca1472fa0d1f168242682e471) by Nikola Milosavljevic).
- upgrade dependencies job (#728) ([7906612](https://github.com/dfinity/dre/commit/79066127f58c852eaf4adda11610e815a426878c) by Nikola Milosavljevic).

## [0.5.0](https://github.com/dfinity/dre/releases/tag/0.5.0) - 2024-08-13

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/d51a1fa228e13af09f7dc15b09e8161ed3dbfb59...0.5.0)</small>

### Features

- [DRE-237] Send desktop notifications when (not) voting with dre tool (#711) ([5d4f8f2](https://github.com/dfinity/dre/commit/5d4f8f2aaacc1e17311db3ffc831ae2dc3c14346) by Saša Tomić).
- adding artifacts (#709) ([13fd109](https://github.com/dfinity/dre/commit/13fd1094fc5451286f062ac969a79e9783b65329) by Nikola Milosavljevic).
- autoupdate ic deps (#706) ([a3b2a5c](https://github.com/dfinity/dre/commit/a3b2a5cd32ab4c521ba2513b80722f204d58db43) by Nikola Milosavljevic).
- subnet authorization with dre (#700) ([6c934f8](https://github.com/dfinity/dre/commit/6c934f82fc87ce16b386f947f9cd6470baf8295a) by Nikola Milosavljevic).
- managing subnet authorization (#697) ([bdc0a7a](https://github.com/dfinity/dre/commit/bdc0a7a807163b51ffe332e2e2ab04cb769ca7dd) by Nikola Milosavljevic).
- add trigger for qualifier workflow (#671) ([3e77d65](https://github.com/dfinity/dre/commit/3e77d65580258f4603cf1fccfdcafa8e75003bf4) by Carly Gundy).
- qualifying as github job (#661) ([5a96588](https://github.com/dfinity/dre/commit/5a96588cb4d31ceb7a1ad0bf7686a02858ca79d5) by Nikola Milosavljevic).
- adding podman (#662) ([5ba79df](https://github.com/dfinity/dre/commit/5ba79dfdcb2cd04f1a5a02a77b95f8cc6980303b) by Nikola Milosavljevic).
- Adding qualificator util (#659) ([40dc9bc](https://github.com/dfinity/dre/commit/40dc9bc407e39d16ec52821ed8627ebea3303e11) by Nikola Milosavljevic).
- qualification via command (#649) ([35303f9](https://github.com/dfinity/dre/commit/35303f9f5106c6685a652e08e21e787867febf08) by Nikola Milosavljevic).
- adding an option for dre to not sync with the nns (#645) ([16abe8d](https://github.com/dfinity/dre/commit/16abe8d8b47c7f2b2c6b1eea0c03b2b5ac973bc2) by Nikola Milosavljevic).
- allowing self update for macos (#624) ([f2559b3](https://github.com/dfinity/dre/commit/f2559b390f0a0ca0f23f80c8fcf644b9a23238c7) by Nikola Milosavljevic).
- upgrading to arbitrary version (#617) ([a808c3f](https://github.com/dfinity/dre/commit/a808c3f77f0a092d5aa22e7ebe71329aefce67b2) by Nikola Milosavljevic).

### Bug Fixes

- adding missing ensurings (#691) ([dde91b0](https://github.com/dfinity/dre/commit/dde91b01e840e9ea3e4b89caf23ec9115e0b7ff3) by Nikola Milosavljevic).
- change inputs format (#672) ([8a1726c](https://github.com/dfinity/dre/commit/8a1726c7e3ef83f4b2c68a4dc8aa29df50090c1f) by Carly Gundy).
- podman setting container without vm (#664) ([7efd87b](https://github.com/dfinity/dre/commit/7efd87b0eac3ebd255be7efe00a3b39b0f9e9fc1) by Nikola Milosavljevic).
- revert separating lib from canisters (#660) ([af99bba](https://github.com/dfinity/dre/commit/af99bba1ea96a1230b3cf5442e8063c7e91d4af6) by Nikola Milosavljevic).
- dry run prints (#653) ([67130e2](https://github.com/dfinity/dre/commit/67130e2a9a70ec52e6004a68ffda742d9811dd17) by Nikola Milosavljevic).
- without confirmation used to run always in dry-run mode (#641) ([8a867b7](https://github.com/dfinity/dre/commit/8a867b7bdf6b899681bf3e2c7838e9a0de0fbb0e) by Nikola Milosavljevic).
- using service account token (#636) ([9e3a81e](https://github.com/dfinity/dre/commit/9e3a81e3ff12e68f187c10eb61efe89fea8697f3) by Nikola Milosavljevic).
- setting correct token for auto update (#635) ([ab60bd5](https://github.com/dfinity/dre/commit/ab60bd5aaf06ba441a083826e12d39482d2472b8) by Nikola Milosavljevic).
- adding full version for auto update (#634) ([07dfdd7](https://github.com/dfinity/dre/commit/07dfdd7912f34f5f69d4c797977d51bbfeaeafad) by Nikola Milosavljevic).
- adding version to auto update (#633) ([80ba727](https://github.com/dfinity/dre/commit/80ba727ed0b1a1e9e134d18d45f5b05c74dcec64) by Nikola Milosavljevic).
- Remove dependency on gitlab for release notes (#614) ([0294110](https://github.com/dfinity/dre/commit/0294110fc369a84b8db2b0b3dc28e20a5cb30f78) by Saša Tomić).
- adding back autodetection of hsm (#619) ([1bd9777](https://github.com/dfinity/dre/commit/1bd97773e85dfe45d8a1707e6240ffd6f8231049) by Nikola Milosavljevic).

### Code Refactoring

- embedding default version excluded subnets (#703) ([6a353f3](https://github.com/dfinity/dre/commit/6a353f37732018532850397ca15789aeebc037c1) by Nikola Milosavljevic).
- replacing boiler plate enum calls with a procedural macro  (#702) ([4aec5fc](https://github.com/dfinity/dre/commit/4aec5fc79c6ef27bb04b62b1e55d2a2579a92d2e) by Nikola Milosavljevic).
- using `autoupdate` action (#638) ([e1783b5](https://github.com/dfinity/dre/commit/e1783b581d630bbc52b878085950ed7bbcaa2eff) by Nikola Milosavljevic).
- major refactoring dre (#581) ([d51a1fa](https://github.com/dfinity/dre/commit/d51a1fa228e13af09f7dc15b09e8161ed3dbfb59) by Nikola Milosavljevic).

## [0.4.3](https://github.com/dfinity/dre/releases/tag/0.4.3) - 2024-07-12

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/e09b98c37ccb4f31dac7b73ba38f40a3fc450d3d...0.4.3)</small>

### Features

- Allow manual workflow dispatch for CI jobs (#594) ([0a4f258](https://github.com/dfinity/dre/commit/0a4f258c3d9b3fd05a18572a0b41f342db6e90a3) by Saša Tomić).
- Move the linear-jira sync from the private release repo to the open DRE repo (#573) ([f48c69d](https://github.com/dfinity/dre/commit/f48c69d46da2dce79202ebdc76bb6c01df36e77c) by Saša Tomić).
- Show the metrics URL to follow the HostOS upgrade progress (#565) ([24c6380](https://github.com/dfinity/dre/commit/24c63809cd05fda8755b5117b26595514a5f06ea) by Saša Tomić).
- Show the date+time when voting, and do not stomp on previous logs (#564) ([5e0866f](https://github.com/dfinity/dre/commit/5e0866f606a871254b0d99132066da121167abec) by Saša Tomić).
-  HostOS rollout improvements (#559) ([c16e3c9](https://github.com/dfinity/dre/commit/c16e3c9ec07a5c0cb7cbc11a2c92f10aa4a2d26e) by Saša Tomić).
- Log noise filter (#348) ([68b109e](https://github.com/dfinity/dre/commit/68b109e4fb00857b5dd8cfbba6e44d02aa0a699c) by Nikola Milosavljevic).
- Update the HostOS release notes to also use merge commits (#550) ([2b5704e](https://github.com/dfinity/dre/commit/2b5704e02e3174a3841b2fdbb9fdc6d85b5e4ff6) by Saša Tomić).
- Only check for updates once per day (#548) ([ce6206c](https://github.com/dfinity/dre/commit/ce6206c47761c0a80f746c67faf76fa7fffbbcd7) by Saša Tomić).
- Speed up get operations by not preparing a neuron and the registry (#541) ([e09b98c](https://github.com/dfinity/dre/commit/e09b98c37ccb4f31dac7b73ba38f40a3fc450d3d) by Saša Tomić).

### Bug Fixes

- Adjust the node label for zh5-dll25 (#595) ([d820e6c](https://github.com/dfinity/dre/commit/d820e6c5b0cbe65366cee2af48e77d35b03224d3) by Saša Tomić).
- Remove duplicate workflow_dispatch ([a6cd1a5](https://github.com/dfinity/dre/commit/a6cd1a559b3f17c54209b223070e3420db04dbcc) by Saša Tomić).
- Update the github actions-runner image tag (#591) ([8230f24](https://github.com/dfinity/dre/commit/8230f24fed074707881beb258d7753d77b14b2b5) by Saša Tomić).
- Add the github keys to the ssh known hosts file (#589) ([9994667](https://github.com/dfinity/dre/commit/9994667f9257350d20c17f8bcbb2d7b3392f39b9) by Saša Tomić).
- self update on non linux machines (#585) ([02283c1](https://github.com/dfinity/dre/commit/02283c10cc62a755e37e9aa9c64413e358fae695) by Nikola Milosavljevic).
- Remove unnecessary Arc::clone (#547) ([10c6eff](https://github.com/dfinity/dre/commit/10c6efff91680244a7001e4b960ae3650f216e40) by Saša Tomić).

### Code Refactoring

- renaming bazel targets (#557) ([1d1d034](https://github.com/dfinity/dre/commit/1d1d0342e0c9d4190ca838250e99ca9b4833d956) by Nikola Milosavljevic).

## [v0.4.2](https://github.com/dfinity/dre/releases/tag/v0.4.2) - 2024-06-26

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/cf40b02935e7954871b91a9ac64501fa8a2cbc8d...v0.4.2)</small>

### Features

- enabling ci checks for release-index.yaml (#534) ([56cd429](https://github.com/dfinity/dre/commit/56cd429eed8b28b27689aecaef58617e7df8d263) by Nikola Milosavljevic).

### Bug Fixes

- asking for update everywhere (#535) ([2faf0cb](https://github.com/dfinity/dre/commit/2faf0cbed0cc8ceba5d2cfc008341ba614a503de) by Nikola Milosavljevic).

### Code Refactoring

- implementing background checks for upgrading (#536) ([3e71733](https://github.com/dfinity/dre/commit/3e71733189f54b42c3afe18acda2f7b469fe96ef) by Nikola Milosavljevic).

## [0.4.1](https://github.com/dfinity/dre/releases/tag/0.4.1) - 2024-06-25

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/ab104f2acb360ad9ea850b2d4e9ffb7b611e8cc9...0.4.1)</small>

### Features

- adding building runner (#529) ([2864b49](https://github.com/dfinity/dre/commit/2864b49b24203b5842420ca31f7a4eb33e4076ce) by Nikola Milosavljevic).
- ci check for release index  (#526) ([63690eb](https://github.com/dfinity/dre/commit/63690eb89adf6fb9324c9ed083324b1a38b2795e) by Nikola Milosavljevic).
- Improve cli operations for generating release notes (#514) ([e00f95d](https://github.com/dfinity/dre/commit/e00f95d876050cbb08f0f320ad6ba1ca0cf6442b) by Saša Tomić).
- Cache public dashboard API response for 1h (#506) ([61bec27](https://github.com/dfinity/dre/commit/61bec279163dda2a31726483df1cefa19282034a) by Saša Tomić).
- Use merge commits if available, and fall back to the non-merge (#502) ([1a78e5a](https://github.com/dfinity/dre/commit/1a78e5a391ad5864f7f9f2461f6b32de4f9adb08) by Saša Tomić).
- Kill the release controller if it gets stuck (#499) ([9baa876](https://github.com/dfinity/dre/commit/9baa876b4c309b1f97c9894b5935ccdae1f45bf9) by Saša Tomić).
- adding job to ensure opentelemetry version (#501) ([a92e88e](https://github.com/dfinity/dre/commit/a92e88eaaa48a24a28f347a512340652d4bc54b9) by Nikola Milosavljevic).
- adding timestamp of last successful sync (#496) ([f029720](https://github.com/dfinity/dre/commit/f029720988584e4a24450be4f83c439c3bf4738e) by Nikola Milosavljevic).
- adding label for api boundary nodes (#483) ([9706b01](https://github.com/dfinity/dre/commit/9706b01b16f0087d3c45230f8eee92aa9a9e13b6) by Nikola Milosavljevic).
- [DRE-178] Accept additional parameters for subnet creation (#478) ([4a4bcd9](https://github.com/dfinity/dre/commit/4a4bcd95ad3d01814a74c05016e2dc11ddb25f16) by Saša Tomić).
- Support dre binary compiled without .git (#477) ([89598dc](https://github.com/dfinity/dre/commit/89598dc7f1a345ca87461663eac93477180c88d2) by Saša Tomić).
- adding mapping of domain from registry (#473) ([303bde2](https://github.com/dfinity/dre/commit/303bde213eff9e343d026f99aa07cf49ffb9a18d) by Nikola Milosavljevic).
- excluding api boundary nodes from available nodes for replacement (#472) ([be7bfb4](https://github.com/dfinity/dre/commit/be7bfb456e908f2f4038825a2d8475b20ad4087c) by Nikola Milosavljevic).
- vote sleep duration (#471) ([a0b48ff](https://github.com/dfinity/dre/commit/a0b48ffc8d92f9efdb3ed9fdb4bfc23fb90ba995) by Nikola Milosavljevic).
- Migrating sns downloader to canister calls (#451) ([9f8fa09](https://github.com/dfinity/dre/commit/9f8fa0929a6040f9380fd9aa44e42d9dd8c8498d) by Nikola Milosavljevic).
- Convenience function for dumping registry records for incorrect rewards (#442) ([00f2e85](https://github.com/dfinity/dre/commit/00f2e857c9e58b2674f5d7a02bd2ab860198d9ea) by Saša Tomić).
- Provide more reward-related information in the registry dump (#440) ([d87334d](https://github.com/dfinity/dre/commit/d87334d106c52cdf1959c2c08b85b3206733f42e) by Saša Tomić).
- implementing checks for correct node rewards set for node operators (#436) ([04c7ba8](https://github.com/dfinity/dre/commit/04c7ba8cae95ba2814df2a9a3bb65856d12f218e) by Nikola Milosavljevic).
- cursors from clickhouse (#433) ([dbd482b](https://github.com/dfinity/dre/commit/dbd482b6fdd8c69573114630313c2b378c522763) by Igor Novgorodov).
- enabling showing progress during self update (#423) ([cd4ef30](https://github.com/dfinity/dre/commit/cd4ef30e995b667eefd84a51c7a804c0eab3937f) by Nikola Milosavljevic).
- adding all known staging nns nodes as defaults (#420) ([ab104f2](https://github.com/dfinity/dre/commit/ab104f2acb360ad9ea850b2d4e9ffb7b611e8cc9) by Nikola Milosavljevic).

### Bug Fixes

- add motivation argument to remove API BN command (#513) ([c833710](https://github.com/dfinity/dre/commit/c833710204fcd54eed4b8410fd087af6ee1d07c5) by r-birkner).
- Remove the double dash in the public dashboard api requests (#511) ([b48ce72](https://github.com/dfinity/dre/commit/b48ce72e8efb1a67f9c75275e78ba8c8bf55d6c0) by Saša Tomić).
- remove duplication of nodes for DeployGuestosToSomeApiBoundaryNodes (#510) ([11a3454](https://github.com/dfinity/dre/commit/11a3454de5fd6f85dc91728dd66184a004081600) by r-birkner).
- Do not try to get auth parameters for ic-admin get-* commands (#508) ([3087304](https://github.com/dfinity/dre/commit/3087304c0a02960209b5e50062f385f1ff8d2509) by Saša Tomić).
- fix typo (#503) ([48855c1](https://github.com/dfinity/dre/commit/48855c13d3fce6136bbfde96a688076c43c9b01b) by Saša Tomić).
- Do not exclude "canister" changes in release notes (#498) ([8dea0aa](https://github.com/dfinity/dre/commit/8dea0aadbfb87c05a669467ac8b4a5d3ea2cb351) by Saša Tomić).
- adding accepting of invalid certs (#491) ([792748e](https://github.com/dfinity/dre/commit/792748e9b42e7db2028e099db8cfab73e24748fa) by Nikola Milosavljevic).
- Do not require an HSM for dry runs (#480) ([30051dd](https://github.com/dfinity/dre/commit/30051dd22d9cc1aabb0c35132ce5c5e6c54ad233) by Saša Tomić).
- fixing tests that didn't run (#481) ([1d38e8e](https://github.com/dfinity/dre/commit/1d38e8eaf8a32ee54733df0898aa60771205bc34) by Nikola Milosavljevic).
- Fix the test for the hostos rollout, add api-boundary-nodes (#479) ([5018651](https://github.com/dfinity/dre/commit/5018651ce42310480655aa0077b8eb752782a6db) by Saša Tomić).
- Add more flexibility to the version regex ([1844452](https://github.com/dfinity/dre/commit/1844452173c713ee6fe05d6ac36176db4288caec) by Saša Tomić).
- fixing typo (#467) ([50e1573](https://github.com/dfinity/dre/commit/50e1573a9373c3bb02486534067d75cd9913b344) by Nikola Milosavljevic).
- Drop the re-classification of commits to "other" in some cases (#463) ([075bde7](https://github.com/dfinity/dre/commit/075bde737fcc9f616b4b07115a49200c1d921a96) by Saša Tomić).
- Delete persisted state file if loading previous contents fails (#459) ([ca08e6a](https://github.com/dfinity/dre/commit/ca08e6af38cad0872dcc394bc44694a269e38bbf) by Saša Tomić).
- not displaying sensitive pin when running help command (#453) ([e16b578](https://github.com/dfinity/dre/commit/e16b57868c210a7b7ee2832119d79fb127cd8c96) by Nikola Milosavljevic).
- Add a time for all agent-rs (canister) clients (#435) ([0c4fa3e](https://github.com/dfinity/dre/commit/0c4fa3e3dbc46037bdd54b848c6357ec2ade2801) by Saša Tomić).
- Fix for the HostOS rollout in groups (#432) ([cd61ab0](https://github.com/dfinity/dre/commit/cd61ab03c7b083da37a2fb6bf4599e7ea3517bb3) by Saša Tomić).

## [0.4.0](https://github.com/dfinity/dre/releases/tag/0.4.0) - 2024-05-24

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/d9dd38cf5b5018a2108bb6ab28d0fff965dbb5ee...0.4.0)</small>

### Features

- log active versions when reconcilling (#413) ([b02ad5f](https://github.com/dfinity/dre/commit/b02ad5fa7f9acc6defe209c2d5af4c3bdd11b483) by Saša Tomić).
- [REL-1517] Auto-pick automation neuron for updating unassigned nodes (#416) ([20640cd](https://github.com/dfinity/dre/commit/20640cd1d00f6223b15ccfe01fc86224b8e6ae96) by Saša Tomić).
- Add a proposals subcommand to get a single proposal by id (#412) ([5bfb885](https://github.com/dfinity/dre/commit/5bfb8856c6d27094cd6c268cbbc824e1b8c3104f) by Saša Tomić).
- Update the DRE cli tool to work with the new IC-OS proposals (#411) ([125b147](https://github.com/dfinity/dre/commit/125b14750ce4ba0e06cf1bb9d6cbeb6eefd4038b) by Saša Tomić).
- Include more info in the registry dump (#410) ([e651338](https://github.com/dfinity/dre/commit/e651338fa8a5274c92f520fdeafae758eaf0169e) by Saša Tomić).
- adding hostos release notes script (#407) ([a99cdb7](https://github.com/dfinity/dre/commit/a99cdb78f9a9750096dc62d3dab01ff869ea2ba0) by Nikola Milosavljevic).
- Automatically update k8s vector container versions (#399) ([47557dc](https://github.com/dfinity/dre/commit/47557dc97d12713bdf743b5d10c458b425e1973a) by Saša Tomić).
- DRE-120 Lazily check for HSM/private key auth (#386) ([cb7be54](https://github.com/dfinity/dre/commit/cb7be549a0c92966435b8b6246bff3f0bcf32df1) by Saša Tomić).
- Better listing proposals (#382) ([649719d](https://github.com/dfinity/dre/commit/649719de5edca8064ca380e58c464874bfa59ca9) by Nikola Milosavljevic).
- listing proposals (#372) ([44261a2](https://github.com/dfinity/dre/commit/44261a2e5147785d1ab0d9bedb509679ab578e39) by Nikola Milosavljevic).
- Bazel remote cache on s3 (#369) ([7ed7fc4](https://github.com/dfinity/dre/commit/7ed7fc4fd9d50a83bd6a9ac54c10368ec916ab07) by Saša Tomić).
- Improve the bazel cache optimization (#368) ([e7b3a8d](https://github.com/dfinity/dre/commit/e7b3a8d41935ea2a3f7cd6b697475f745b661105) by Saša Tomić).
- adding using of public dashboard for mainnet for health statuses (#362) ([70b4c5f](https://github.com/dfinity/dre/commit/70b4c5fac3d35df2bd3888ac072754416244a68e) by Nikola Milosavljevic).
- adding v1 structure for node labels in dre repo (#361) ([f6bda6e](https://github.com/dfinity/dre/commit/f6bda6e545e13e55559885d58fad50a260153285) by Nikola Milosavljevic).
- adding api boundary nodes to registry dump command (#357) ([42de821](https://github.com/dfinity/dre/commit/42de821e29f2ea914650877c1a682cac99c9de27) by Nikola Milosavljevic).
- adding elastic backup job (#343) ([8ba3f20](https://github.com/dfinity/dre/commit/8ba3f201dbc764078b88512375730255824b1c90) by Nikola Milosavljevic).
- implemented automatic updates for dre tool (#341) ([cd60325](https://github.com/dfinity/dre/commit/cd60325481b1dbca192e44f9b18fdea82d729681) by Nikola Milosavljevic).
- Automation script for removing nodes from subnets (#328) ([535d731](https://github.com/dfinity/dre/commit/535d7316738694b1ec8a39c3d058efaf9679d59a) by Saša Tomić).
- implementing completions (#319) ([b8bb7e0](https://github.com/dfinity/dre/commit/b8bb7e0e60247d30f69aee8809d6c984d2efeed0) by Nikola Milosavljevic).
- Support for targetting arbitrary IC network on the cli (#302) ([d1ac15b](https://github.com/dfinity/dre/commit/d1ac15b30eadb1a19372fd5ea13df161cc5e9977) by Saša Tomić).
- Maintain up to 1 open MR in the k8s repo per DRE branch (#298) ([e99dd08](https://github.com/dfinity/dre/commit/e99dd088ad143635dfa186216ada03d014ca677f) by Saša Tomić).
- If possible reuse the MR in the k8s repo to update container images (#290) ([127ef28](https://github.com/dfinity/dre/commit/127ef28a8bce442a4a775eea078395214a77102a) by Saša Tomić).
- adding job for building dashboard (#278) ([0abe44a](https://github.com/dfinity/dre/commit/0abe44a0aa883cc653517e28f734bdcdd0f80e8a) by Nikola Milosavljevic).

### Bug Fixes

- Show DC in the node label/name (#406) ([f48e685](https://github.com/dfinity/dre/commit/f48e685be32251ad9b2bf50be507e71fa976c238) by Saša Tomić).
- Add compatibility in fetching trustworthy metrics from old subnet versions (#401) ([77e39d2](https://github.com/dfinity/dre/commit/77e39d2609a421300e8289953894b5b056852ccc) by Saša Tomić).
- Set the default socket timeout to 60s ([55c2abb](https://github.com/dfinity/dre/commit/55c2abba5e565c2591f7f1733a364eb5c08051e7) by Saša Tomić).
- [DRE-166] use requests instead of urllib, to handle timeouts (#398) ([4838df8](https://github.com/dfinity/dre/commit/4838df81bed2d3d1ef553367424c7d200f66e868) by Saša Tomić).
- [DRE-166] Add timeouts to the URL requests (#395) ([ec25f69](https://github.com/dfinity/dre/commit/ec25f6963f2e27f01ae8dd5c80d389e3d629f5ee) by Saša Tomić).
- Only update posts if different and can_edit (#391) ([846345a](https://github.com/dfinity/dre/commit/846345acb80e711f6f8de41f0bfecec7bfd28285) by Saša Tomić).
- adding missing mapping for unassigned status from public dashboard (#379) ([2e14819](https://github.com/dfinity/dre/commit/2e148192156e2ec60c30c696028e8f8ff86df8ae) by Nikola Milosavljevic).
- Do not check the health of the force-included nodes (#377) ([2ab31fb](https://github.com/dfinity/dre/commit/2ab31fb172b258d3fc469397d10d0d5b4534dbc2) by Saša Tomić).
- Ensure "interface-owners" changes are included (#366) ([f517739](https://github.com/dfinity/dre/commit/f51773948fd2af3a3f439a45c37b56b06e94c858) by Saša Tomić).
- build dashboard on PR only if it was referenced in the branch (#308) ([c072728](https://github.com/dfinity/dre/commit/c07272843431b26985a38001cdb9db30913223b1) by Nikola Milosavljevic).
- Openssl missing and dry run in CI (#303) ([b8bc57e](https://github.com/dfinity/dre/commit/b8bc57eed6ab5d1f21f4250b5a6890d73b4cb270) by Nikola Milosavljevic).
- fixing condition for pushing of dashboard image (#296) ([283e346](https://github.com/dfinity/dre/commit/283e34636d2f0c14b81adb69d0938f567b79c6b0) by Nikola Milosavljevic).
- k8s repo make MR (#297) ([2c0e676](https://github.com/dfinity/dre/commit/2c0e6764f00950971908e74ea71dfbd525ace8a4) by Saša Tomić).
- Fix bin/mk-release to not tag versions with "vv" in some cases (#288) ([d9dd38c](https://github.com/dfinity/dre/commit/d9dd38cf5b5018a2108bb6ab28d0fff965dbb5ee) by Saša Tomić).

### Code Refactoring

- migrating from using gitlab for node labels to using github (#384) ([dee7f89](https://github.com/dfinity/dre/commit/dee7f899ffb1d94c10016d3fd813a9addc978d38) by Nikola Milosavljevic).

## [0.3.2](https://github.com/dfinity/dre/releases/tag/0.3.2) - 2024-04-03

<small>[Compare with first commit](https://github.com/dfinity/dre/compare/a89c1826bf57646dd88d40f619bc216b3961cf64...0.3.2)</small>

### Added

- Add features for release rc--2024-03-20_23-01 (#257) ([a89c182](https://github.com/dfinity/dre/commit/a89c1826bf57646dd88d40f619bc216b3961cf64) by Luka Skugor).

### Fixed

- fix ([f0c44d7](https://github.com/dfinity/dre/commit/f0c44d767050687b7e268977a5a32b20584c2222) by Saša Tomić).
- fix command order ([f3d6ea9](https://github.com/dfinity/dre/commit/f3d6ea9d671457bcfaeaf623a33342650d7c34b6) by Saša Tomić).
- fix docker image run issues (#268) ([bb9acec](https://github.com/dfinity/dre/commit/bb9acecaad2854d0f01b6bd2d1177b900aa86b73) by Luka Skugor).

### Changed

- Changes by create-pull-request action (#245) ([278cc64](https://github.com/dfinity/dre/commit/278cc64a87850edb30a58b613522cd69e0693ae0) by sa-github-api).

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
