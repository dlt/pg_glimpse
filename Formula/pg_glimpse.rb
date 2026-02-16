class PgGlimpse < Formula
  desc "Terminal-based PostgreSQL monitoring tool with live TUI"
  homepage "https://github.com/dlt/pg_glimpse"
  license "MIT"
  version "0.7.0"

  on_macos do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "9aad5b9f4d1448182100e194c4fe16eb1c3662a5429bd5a493ac44cb98db5dfc"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "ea09daa5e871cd75e3aad1e5c9f6a2700d9b3beb63cd37f31d770737ab051a5a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "18ae65145015aaf8c13548b64e7bd13d2171fff4645cad5deaf3071316f1d237"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "f9e14441ba89e069e3e4c699d6d06dc0dd09989d675f4c26ca308cdd28a9ba6e"
    end
  end

  def install
    bin.install "pg_glimpse"
  end

  test do
    assert_match "pg_glimpse", shell_output("#{bin}/pg_glimpse --version")
  end
end
