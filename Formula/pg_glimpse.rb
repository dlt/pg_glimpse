class PgGlimpse < Formula
  desc "Terminal-based PostgreSQL monitoring tool with live TUI"
  homepage "https://github.com/dlt/pg_glimpse"
  license "MIT"
  version "0.2.9"

  on_macos do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "4d0036e90c8e54cc265212c6fc0527869b2df258a17a15539d2f21f6ab590188"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "1e7c0f0a8f973fdab59ba887b665075f561b17d9b71c39db449002ebfa95b582"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "798216029cf74493c37d14b1feb85c447cdf3f8190ae27eee050684cb5368669"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "48284a8a2d4d4d8a5d217599a0e925ee49d9288e89f8681baa8cc39fd2ebd26a"
    end
  end

  def install
    bin.install "pg_glimpse"
  end

  test do
    assert_match "pg_glimpse", shell_output("#{bin}/pg_glimpse --version")
  end
end
