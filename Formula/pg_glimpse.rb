class PgGlimpse < Formula
  desc "Terminal-based PostgreSQL monitoring tool with live TUI"
  homepage "https://github.com/dlt/pg_glimpse"
  license "MIT"
  version "0.2.10"

  on_macos do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "c2c9d8007bf68d8c2bb57004fb33eefcd16b8b7e19e99b5187553eedd0ce7d4a"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "02027ddbf7698a6318221e53750bbbf620c82ccd97322fc49e3b0784d6c6a1cf"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "c00a053837a0a1fc30bb9907c92768320753c997fab1377d788a43f4fa61991a"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "74b7ff3623f335c55717cd088b806b1ed0293240aed65da310ee32f054996fa1"
    end
  end

  def install
    bin.install "pg_glimpse"
  end

  test do
    assert_match "pg_glimpse", shell_output("#{bin}/pg_glimpse --version")
  end
end
