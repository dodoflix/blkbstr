# Installing on Linux

Each release publishes an AppImage, a `.deb` and an `.rpm`, plus apt and dnf repositories and an AUR package so updates arrive through the package manager. The packages hold the app only; the privileged helper and the engine still come from [packaging/linux](../packaging/linux/README.md) and the setup wizard.

## Debian, Ubuntu

```sh
sudo curl -fsSL https://dodoflix.github.io/blkbstr/key.asc -o /etc/apt/keyrings/blkbstr.asc
echo 'deb [signed-by=/etc/apt/keyrings/blkbstr.asc] https://dodoflix.github.io/blkbstr/apt ./' | sudo tee /etc/apt/sources.list.d/blkbstr.list
sudo apt update && sudo apt install blockbuster
```

## Fedora, RHEL, openSUSE

```sh
sudo curl -fsSL https://dodoflix.github.io/blkbstr/rpm/blkbstr.repo -o /etc/yum.repos.d/blkbstr.repo
sudo dnf install blockbuster
```

On openSUSE: `sudo zypper addrepo https://dodoflix.github.io/blkbstr/rpm/blkbstr.repo && sudo zypper install blockbuster`.

## Arch, CachyOS, Manjaro

```sh
yay -S blkbstr-bin
```

## Verifying a download

Every release carries `SHA256SUMS`, signed with the repository key (`key.asc` above), and a build provenance attestation for each file.

```sh
curl -fsSLO https://dodoflix.github.io/blkbstr/key.asc && gpg --import key.asc
gpg --verify SHA256SUMS.asc SHA256SUMS && sha256sum -c --ignore-missing SHA256SUMS
gh attestation verify Blockbuster_*_amd64.deb --repo dodoflix/blkbstr
```
