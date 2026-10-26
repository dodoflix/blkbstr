# Changelog

## [0.3.0](https://github.com/dodoflix/blkbstr/compare/v0.2.2...v0.3.0) (2026-10-26)


### Features

* install the helper from the app on Linux ([#64](https://github.com/dodoflix/blkbstr/issues/64)) ([2651e9d](https://github.com/dodoflix/blkbstr/commit/2651e9d0b9ebc6915845489780bb6ad149ba9200))


### Bug fixes

* download the engine in the elevated step and keep the helper across app updates ([#61](https://github.com/dodoflix/blkbstr/issues/61)) ([a767ec1](https://github.com/dodoflix/blkbstr/commit/a767ec115b1458b56baaebe22a88cebce30b1f81))

## [0.2.2](https://github.com/dodoflix/blkbstr/compare/v0.2.1...v0.2.2) (2026-10-25)


### Bug fixes

* **daemon:** exclude the engine archive from Windows Security before reading it ([#59](https://github.com/dodoflix/blkbstr/issues/59)) ([16486d5](https://github.com/dodoflix/blkbstr/commit/16486d59b9f1be5b7b8efe17d6cb341417b1dafa))

## [0.2.1](https://github.com/dodoflix/blkbstr/compare/v0.2.0...v0.2.1) (2026-10-25)


### Bug fixes

* **daemon:** stop Windows Security from removing the engine during install ([#56](https://github.com/dodoflix/blkbstr/issues/56)) ([3352e1a](https://github.com/dodoflix/blkbstr/commit/3352e1a3561163e4985ac695d97aff1f74bdcb2a))
* use the Blockbuster icon and artwork in the Windows installer ([#58](https://github.com/dodoflix/blkbstr/issues/58)) ([3e590a8](https://github.com/dodoflix/blkbstr/commit/3e590a8d95c64e5e0189aa23f01cdfceb205abe4))

## [0.2.0](https://github.com/dodoflix/blkbstr/compare/v0.1.1...v0.2.0) (2026-10-25)


### Features

* check for app updates on start and every six hours ([#49](https://github.com/dodoflix/blkbstr/issues/49)) ([c3fcdad](https://github.com/dodoflix/blkbstr/commit/c3fcdadce8d8bcdfc9dea87d7bc476e437942a14))
* keep an app log and record panics, elevation and install steps ([#52](https://github.com/dodoflix/blkbstr/issues/52)) ([e7c0cdc](https://github.com/dodoflix/blkbstr/commit/e7c0cdc901c6e5c1fc3517f2e51ef8bac94c8dfa))
* notify about app updates and stay in the tray when the window is closed ([#53](https://github.com/dodoflix/blkbstr/issues/53)) ([2b070a8](https://github.com/dodoflix/blkbstr/commit/2b070a8c16a2fdc5fb9035be5069c26f6f98232f))


### Bug fixes

* show why the elevated engine install failed on Windows ([#50](https://github.com/dodoflix/blkbstr/issues/50)) ([550c995](https://github.com/dodoflix/blkbstr/commit/550c9951d5bc10a5505bd0efa2206a1906cba77a))

## [0.1.1](https://github.com/dodoflix/blkbstr/compare/v0.1.0...v0.1.1) (2026-10-25)


### Bug fixes

* **daemon:** look for the engine again after startup ([#47](https://github.com/dodoflix/blkbstr/issues/47)) ([14f04ba](https://github.com/dodoflix/blkbstr/commit/14f04ba7d074d24002cf0dbcd9c4721a4de89cd8))

## 0.1.0 (2026-10-25)


### Features

* add app and engine updates and a system tray icon ([#45](https://github.com/dodoflix/blkbstr/issues/45)) ([bead964](https://github.com/dodoflix/blkbstr/commit/bead964a529f117b720313f7b8f739f20c8a3e43))
* add AppArmor and SELinux profiles; fix the socket chgrp under a trimmed capability set ([9ed7115](https://github.com/dodoflix/blkbstr/commit/9ed71159dd6b2047ae3d928d012579ce9de26939))
* add auto-configuration over a ranked candidate list ([98c9f7c](https://github.com/dodoflix/blkbstr/commit/98c9f7c6cefa6f92cb5c3547ebf76c02ec01e6b0))
* add confirm-or-revert to trial runs ([f7625f0](https://github.com/dodoflix/blkbstr/commit/f7625f02681ddd8e1de25ba172906747c41d7a13))
* add the initial Blockbuster app, a desktop GUI for zapret2 ([4771b84](https://github.com/dodoflix/blkbstr/commit/4771b84249ab5c676b154c02ac1c8c3b7b90ce92))
* add the log viewer and diagnostics export ([14aee89](https://github.com/dodoflix/blkbstr/commit/14aee896a9d5188bf6ecae5eaac62680e9b77225))
* add the reachability check with a per-site verdict ([1780ff4](https://github.com/dodoflix/blkbstr/commit/1780ff4402545b4354098573666c3f5291340925))
* add the Windows backend, service and installer ([#44](https://github.com/dodoflix/blkbstr/issues/44)) ([c08c019](https://github.com/dodoflix/blkbstr/commit/c08c019473f85dfe175340e160b242e490f4b663))
* detect what is already on the machine, and show it ([ee27e84](https://github.com/dodoflix/blkbstr/commit/ee27e847c5ce76f7e85cdb262a6ddc7cbf483ec0))
* install the engine from the wizard and import an existing zapret2 config ([#43](https://github.com/dodoflix/blkbstr/issues/43)) ([c9bbd76](https://github.com/dodoflix/blkbstr/commit/c9bbd76bc33309c42982ef77576434b1aea3776b))
* learned and own host lists, categorized site checks ([#46](https://github.com/dodoflix/blkbstr/issues/46)) ([f7f0a3d](https://github.com/dodoflix/blkbstr/commit/f7f0a3ddb238687d26c0e2de3bb22066f975648d))
* redesign the interface on its own design system ([#33](https://github.com/dodoflix/blkbstr/issues/33)) ([793973f](https://github.com/dodoflix/blkbstr/commit/793973f6e4fd8cf826850acd41385081892373a6))
* supervise the engine and survive a reboot ([e87d1ae](https://github.com/dodoflix/blkbstr/commit/e87d1aeb69115b6c5a5285a6bd01bb9c2767e529))
* use the Blockbuster icon for the app, the favicon and the platform icon sets ([5f444a1](https://github.com/dodoflix/blkbstr/commit/5f444a1390d65a9ac3f6791bbf0ec5f99882afe5))


### Bug fixes

* allow the socket and /proc/self/exe in the AppArmor profile ([00f2178](https://github.com/dodoflix/blkbstr/commit/00f2178ebc0d9411162242652c832c05897f78a1))
* drop MemoryDenyWriteExecute and relax ProtectHome in the unit ([7d872bd](https://github.com/dodoflix/blkbstr/commit/7d872bdbf7fad3a1ce993af323e8fa7fc42327e8))
* keep scrollbars behind the sidebar and animate their hover ([#41](https://github.com/dodoflix/blkbstr/issues/41)) ([9f8c725](https://github.com/dodoflix/blkbstr/commit/9f8c72582f6f94b6e3cb714437415eb65d15408a))
* load zapret2's lua library and validate with --intercept=0 ([#18](https://github.com/dodoflix/blkbstr/issues/18)) ([ae3252a](https://github.com/dodoflix/blkbstr/commit/ae3252a0f436763c141ff33dd9610f86f6600a9e))
* open the window on NVIDIA under Wayland; add service control ([10fe1b9](https://github.com/dodoflix/blkbstr/commit/10fe1b9efabd157112cf26eeeb5a9261882cabe8))


### Documentation

* add a legal notice and a new README, and trim comments ([#42](https://github.com/dodoflix/blkbstr/issues/42)) ([8fa50d9](https://github.com/dodoflix/blkbstr/commit/8fa50d9e12710e609592180366faa8a4155024bf))
* put the README logo in the heading instead of floating it ([7a19116](https://github.com/dodoflix/blkbstr/commit/7a1911609091a092aa7a22dcf35e580602817cb6))
* record this milestone's changes in the changelog ([56ed569](https://github.com/dodoflix/blkbstr/commit/56ed56988b848520e409570dcb2079118a0b8a03))
* say zapret2 in the crate descriptions, the UI tagline and the plan ([6d3e108](https://github.com/dodoflix/blkbstr/commit/6d3e108f5bb19d4ade28fe359c32f534cc8fb1ee))

## Changelog
