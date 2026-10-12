# The Wipemark Linux builder: Ubuntu 24.04, the pinned Rust, GPUI's build
# dependencies and the AppImage tools — the one machine a release AppImage and
# .deb are built on (D600).
#
# Who asked: the owner, 2026-10-12 — package Wipemark as MnemoVi is packaged.
# This file is MnemoVi's ci/linux-builder/Dockerfile cut down to what a GPUI
# application with llama.cpp needs: no Node, no WebKit, no GStreamer, no
# shaderc (llama.cpp comes prebuilt, crates/wipemark-llama-sys/src/pin.rs),
# no linuxdeploy (D601).
#
# WHY 24.04. The prebuilt llama.cpp the release links was built on Ubuntu
# 24.04 and needs glibc 2.38 (its Linux archive's PROVENANCE.txt says 2.39);
# a base older than that cannot link it, and a base newer would raise the
# floor for nothing. The floor is checked, not assumed:
# scripts/release/linux/glibc-floor.sh walks every ELF in the AppDir.
#
# THE BASE IS PINNED BY DIGEST, NOT BY TAG — `ubuntu:24.04` is republished on
# every point release. Bump it deliberately; the image tag (image-ref.sh) is
# derived from this file, so a bump is a new image, never a mutated one.
ARG UBUNTU_DIGEST=sha256:561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea
FROM ubuntu@${UBUNTU_DIGEST}
ENV DEBIAN_FRONTEND=noninteractive

# The Rust the workspace pins (rust-toolchain.toml). Stated here, not read at
# build time, so the image's name changes when the pin does: image-ref.sh
# hashes this file, and scripts/release/linux/build.sh refuses an image whose
# rustc is not the one rust-toolchain.toml names.
ARG RUST_VERSION=1.95.0

# System packages.
#   build          build-essential pkg-config git curl ca-certificates file
#   GPUI on Linux  the list .github/workflows/gate.yml installs:
#                  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev
#                  libxcb1-dev libx11-xcb-dev libx11-dev libfontconfig1-dev
#                  libfreetype-dev libssl-dev libzstd-dev libasound2-dev
#                  libgtk-3-dev libayatana-appindicator3-dev
#   llama.cpp      libvulkan-dev (the loader the prebuilt Vulkan backend
#                  NEEDs; the build links nothing against it) and libgomp1,
#                  which the AppImage CARRIES (excludelist-policy, SHIP)
#   packaging      patchelf (the binaries' $ORIGIN rpath), binutils (objdump,
#                  readelf for the floors and the closure), desktop-file-utils
#                  (desktop-file-validate), appstream (appstreamcli validate),
#                  python3 (the third-party notices from cargo metadata),
#                  squashfs-tools (unsquashfs, to read a finished AppImage back)
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential pkg-config git curl ca-certificates file \
      libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
      libxcb1-dev libx11-xcb-dev libx11-dev \
      libfontconfig1-dev libfreetype-dev libssl-dev libzstd-dev libasound2-dev \
      libgtk-3-dev libayatana-appindicator3-dev \
      libvulkan-dev libgomp1 \
      patchelf binutils desktop-file-utils appstream python3 squashfs-tools \
 && rm -rf /var/lib/apt/lists/*

# The pinned tools, fetched and checked on the host by fetch-tools.sh and
# checked again here, so a stale or tampered vendor/ cannot reach an image.
COPY vendor/ /opt/wipemark-tools/
COPY tools.sha256 /opt/wipemark-tools/
RUN cd /opt/wipemark-tools \
 && awk '!/^#/ && NF {print $1 "  " $2}' tools.sha256 | sha256sum -c - \
 && chmod 0755 appimagetool-x86_64.AppImage rustup-init \
 && chmod 0644 runtime-x86_64

# Rust, in the image, at the pin, with the components rust-toolchain.toml
# lists — otherwise rustup tries to add clippy and rustfmt at the first cargo
# call, into a RUSTUP_HOME the build user cannot write. Only the caches mount
# at run time (run-build.sh); mounting the host's rustup would replace this
# pin with whatever the host has.
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
RUN /opt/wipemark-tools/rustup-init -y --no-modify-path --profile minimal \
      --default-toolchain "${RUST_VERSION}" -c rustfmt -c clippy \
 && chmod -R a+rX /opt/cargo /opt/rustup \
 && rustc --version && cargo --version

# An AppImage is a FUSE mount, and a container has no /dev/fuse: every
# AppImage run in here (appimagetool itself, and the gates' --version) unpacks
# itself instead.
ENV APPIMAGE_EXTRACT_AND_RUN=1

# Build as the invoking user, not root: the checkout and the caches mounted in
# belong to the host user, and root-owned files in them would break the
# owner's own cargo afterwards. noble has a uid-1000 `ubuntu`; free the id.
ARG BUILDER_UID=1000
ARG BUILDER_GID=1000
RUN userdel -r ubuntu 2>/dev/null || true; \
    groupadd -g "${BUILDER_GID}" builder 2>/dev/null || true; \
    useradd -m -u "${BUILDER_UID}" -g "${BUILDER_GID}" -s /bin/bash builder
USER ${BUILDER_UID}:${BUILDER_GID}
WORKDIR /work
CMD ["bash"]
