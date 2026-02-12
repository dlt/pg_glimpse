class PgGlimpse < Formula
  desc "Terminal-based PostgreSQL monitoring tool with live TUI"
  homepage "https://github.com/dlt/pg_glimpse"
  license "MIT"
  version "0.5.2"

  on_macos do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "62a0eebd96867f0e70914d6d6d4ba169ca95d50d2318b1458922afb6189785b7"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "f9014771e08cb2a5576124ec46f87bdbaf255992dc1ad58acacba3d5d96d4131"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "f64d8298bcf61289648cfa3c1060d8da98f46e3ce2695e51361812fdbd5a85d1"
    end

    on_intel do
      url "https://github.com/dlt/pg_glimpse/releases/download/v#{version}/pg_glimpse-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "a94c90923dbadfd6d70adf771891fd629301ea83f0b65295e62f06fde9196534"
    end
  end

  def install
    bin.install "pg_glimpse"
  end

  test do
    assert_match "pg_glimpse", shell_output("#{bin}/pg_glimpse --version")
  end
end
