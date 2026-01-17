# Homebrew formula for ctx
# To use: move this to Formula/ctx.rb in the same repo
# Users install with: brew install vladisov/ctx/ctx

class Ctx < Formula
  desc "Curate context packs for LLMs"
  homepage "https://github.com/vladisov/ctx"
  version "0.3.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/vladisov/ctx/releases/download/v#{version}/ctx-aarch64-apple-darwin.tar.gz"
      sha256 "d268e6685f75df3162e9bae01f37e1c68af807b1616495a7573e354197dad490"

      def install
        bin.install "ctx"
      end
    end

    on_intel do
      url "https://github.com/vladisov/ctx/releases/download/v#{version}/ctx-x86_64-apple-darwin.tar.gz"
      sha256 "97a1f1f150b143ff5704887c5966496e169923425b3d48b271cbc7089e0218d3"

      def install
        bin.install "ctx"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/vladisov/ctx/releases/download/v#{version}/ctx-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "6f210c03983683b41e7febe050daba067ceaa323a1dc0fbc56ba1f655e592eb1"

      def install
        bin.install "ctx"
      end
    end
  end

  test do
    assert_match "ctx", shell_output("#{bin}/ctx --version")
  end
end
