class PgGlimpse < Formula
  desc "Terminal-based PostgreSQL monitoring tool with live TUI"
  homepage "https://github.com/dlt/pg_glimpse"
  license "MIT"
  version "0.2.2"

  on_macos do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "4f13bab5a4babae487eff312524266d7bcde5c4b8f0994d6668e8369417ea090"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0da9c5727942503dcdcb661bacca4135c38e01b49e446c7e8aa88409516de3ae"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "2c0bab97c1d5488c627bc9ff09fa6a95d58b089f6f8f4c5b38be84602bdeae4d"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "72f4dc1f4f1b5ba9a97d3b293e248df9a9ed5e12b43a27520601c87cfbf6322b"
    end
  end

  def install
    bin.install "pg_glimpse"
  end

  test do
    assert_match "pg_glimpse", shell_output("#{bin}/pg_glimpse --version")
  end
end
