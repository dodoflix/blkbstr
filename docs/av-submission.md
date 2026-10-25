# Antivirus false positives

Blockbuster drives a tool that injects packets, spoofs TTLs and loads a network driver. To a
heuristic engine that is malware, so some vendors will flag the installer, `blkbstrd.exe` or the
engine's files. Two different things get flagged and they are fixed differently:

- **Our files** (the installer, `Blockbuster.exe`, `blkbstrd.exe`): signed by us, so a vendor can be
  asked to clear them, and a signature carries reputation from release to release.
- **Upstream's files** (`winws2.exe`, `cygwin1.dll`, `WinDivert.dll`, `WinDivert64.sys`): not ours
  to submit as the author, but the same vendor forms accept anyone's file, and a flag on these
  breaks the engine, not the app. Report them too, naming the upstream project.

## Before each release

1. Sign the release candidate once a certificate exists. Until then, releases are unsigned,
   which is judged on bytes alone and flagged far more often, so expect more reports from users.
2. Upload the installer to [VirusTotal](https://www.virustotal.com) and note which engines flag it.
   That list is the work list below.
3. Submit it to each vendor that flagged it, and to the ones people actually run:

   | Vendor | Where |
   | --- | --- |
   | Microsoft Defender | https://www.microsoft.com/en-us/wdsi/filesubmission (choose "software developer") |
   | Kaspersky | https://opentip.kaspersky.com |
   | Avast and AVG | https://www.avast.com/false-positive-file-form.php |
   | Norton | https://submit.norton.com |

   Other vendors move their forms often; search "<vendor> false positive submission" rather than
   trusting an old link.
4. In every submission give the same facts: the SHA-256 from `SHA256SUMS-windows.txt`, the project
   URL, that the build is signed and attested, that the source is public, and what the program does
   (it drives the open-source zapret2 engine, which uses the WinDivert driver to rewrite the user's
   own outgoing packets). Do not claim it does nothing suspicious; say exactly what it does.
5. Do not publish the release until Defender has cleared it. Defender is what most users have, and
   it is the one that removes the file silently.

Re-submit when a flag comes back. A new certificate or a new major version resets reputation.

## What to tell a user whose antivirus blocked it

If an installer or file disappears, check the antivirus quarantine first. Restore it, and exclude
these locations so it is not removed again:

- `%ProgramFiles%\Blockbuster`
- `%ProgramFiles%\zapret2`

Only restore a file whose signature is valid and whose SHA-256 matches `SHA256SUMS-windows.txt` from
the release page:

```powershell
Get-AuthenticodeSignature .\Blockbuster_*_x64-setup.exe
Get-FileHash .\Blockbuster_*_x64-setup.exe -Algorithm SHA256
```

A Windows Security exclusion is added under Virus & threat protection, Manage settings, Exclusions.
Third-party kernel software, mostly other security products, can also conflict with WinDivert; if
Windows crashes with a blue screen after starting the engine, stop the helper and report which
products are installed.
