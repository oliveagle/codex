# AGENTS.ole.md - oliveagle/codex Fork Specific Requirements

This file contains requirements and workflows specific to the oliveagle/codex fork.
**Note:** This file should NOT be included in merge requests to upstream (openai/codex).

## Linux AMD64 Build and Deployment

### Build Strategy
- Build Linux AMD64 binaries on GitHub Actions using `ubuntu-22.04` runner (glibc 2.35)
- This ensures compatibility with:
  - Debian 12 (glibc 2.36)
  - Ubuntu 22.04+ (glibc 2.35+)
  - Ubuntu 24.04 (glibc 2.39)
- Use the `build-linux-amd64.yml` workflow for building
- Do NOT build on individual Linux machines - use GitHub Actions and deploy artifacts

### Deployment Targets
- **oleNucBoxEVOX2** (10.126.126.4): Ubuntu 24.04, x86_64
- **oleNas01** (10.126.126.6): Debian 12, x86_64

### Deployment Workflow
1. Trigger GitHub Actions workflow: `gh workflow run build-linux-amd64.yml`
2. Wait for build completion (~35-40 minutes)
3. Download artifact: `gh run download <run-id> --name codex-linux-amd64-tarball`
4. Extract and deploy to target machines:
   ```bash
   # Extract binary
   tar xzf codex-linux-amd64.tar.gz
   
   # Deploy to oleNucBoxEVOX2
   scp codex oleNucBoxEVOX2:~/.local/bin/
   ssh oleNucBoxEVOX2 "chmod +x ~/.local/bin/codex"
   
   # Deploy to oleNas01
   scp codex oleNas01:~/.local/bin/
   ssh oleNas01 "chmod +x ~/.local/bin/codex"
   ```

### Why Not Build Locally?
- Avoids glibc version compatibility issues
- Consistent build environment
- Faster than building on remote machines
- Artifacts are cached and reproducible

## Upstream MR Guidelines
When creating merge requests to openai/codex:
- Exclude this file (AGENTS.ole.md) from the PR
- Exclude any fork-specific workflows or configurations
- Only include changes that benefit the upstream project
