class PgGlimpse < Formula
  desc "Terminal-based PostgreSQL monitoring tool with live TUI"
  homepage "https://github.com/dlt/pg_glimpse"
  license "MIT"
  version "0.1.1"

  on_macos do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "de209fd9312d2abefb4e117bb1b5b5e7b7f0c6dfaa6142fc12a2fa4182e30e83"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "dd5a20d1c1c795d204705d2cf15228406607c284cb5f543606d07dca9edd625a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "497015172db03afb457932b6026bfdb989e1a018d704f5b36a5c35f05d4636da"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "53511603decbdf0f36c63eda3ae7660212372765236d01a34368c71fc4f5426d"
    end
  end

  def install
    bin.install "pg_glimpse"
  end

  test do
    assert_match "pg_glimpse", shell_output("#{bin}/pg_glimpse --version")
  end
end
