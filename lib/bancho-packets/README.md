# bancho-packets

**osu! bancho packet Reading & Writing library.**

**[docs https://docs.rs/bancho-packets](https://docs.rs/bancho-packets)**

### Usage

Add to your `cargo.toml`
```toml
[dependencies]
bancho-packets = "4"
```

Or run the following Cargo command in your project directory:
```bash
cargo add bancho-packets
```




### Examples

see more: [examples](examples), [src/tests.rs](src/tests.rs)

**Reading packets**

```rust
use bancho_packets::{PacketReader, PayloadReader};

// Packets from osu! bancho
let data = &[
    4, 0, 0, 0, 0, 0, 0, 24, 0, 0, 19, 0, 0, 0, 11, 17, 72, 101, 108, 108,
    111, 44, 32, 87, 111, 114, 108, 100, 33, 240, 159, 146, 150, 4, 0, 0,
    0, 0, 0, 0, 24, 0, 0, 18, 0, 0, 0, 11, 16, 229, 147, 136, 229, 147,
    136, 227, 128, 144, 240, 159, 152, 131, 227, 128, 145, 104, 0, 0, 0, 0,
    0, 0, 24, 0, 0, 23, 0, 0, 0, 11, 21, 232, 175, 187, 229, 143, 150, 229,
    174, 140, 228, 186, 134, 239, 188, 129, 239, 188, 129, 226, 156, 168,
];

// Create reader
let mut reader = PacketReader::new(data);

// Read packets
while let Some(packet) = reader.next() {
    print!("packet id: {:?}: ", packet.id);
    match packet.payload {
        Some(payload) => {
            // Read payload
            let mut payload_reader = PayloadReader::new(payload);
            println!("payload: {:?}", payload_reader.read::<String>());
        },
        None => println!("Non-payload"),
    }
}

/* Results
  packet id: OSU_PING: Non-payload
  packet id: BANCHO_NOTIFICATION: payload: Some("Hello, World!💖")
  packet id: OSU_PING: Non-payload
  packet id: BANCHO_NOTIFICATION: payload: Some("哈哈【😃】")
  packet id: BANCHO_ACCOUNT_RESTRICTED: Non-payload
  packet id: BANCHO_NOTIFICATION: payload: Some("读取完了！！✨")
*/
```


**Writing packets**

```rust
use bancho_packets::*;

// Single packet
let login_reply_from_server = server::login_reply(LoginResult::Failed(
    LoginFailedResaon::InvalidCredentials,
));
let serverside_notification = server::notification("hello");

// Multiple packets with Builder
let packets = PacketBuilder::new()
    .add(server::login_reply(LoginResult::Success(1000)))
    .add(server::protocol_version(19))
    .add(server::notification("Welcome to osu!"))
    .add(server::main_menu_icon("https://image.png", "https://url.link"))
    .add(server::silence_end(0))
    .add(server::channel_info_end())
    .build();

```

**Build your own packet**

```rust
use bancho_packets::*;

// Build simple packet
let number_data: i32 = 1;
let packet = packet!(PacketId::BANCHO_MATCH_PLAYER_SKIPPED, number_data)

// Complex
pub fn user_stats(
    user_id: i32,
    action: u8,
    info: String,
    beatmap_md5: String,
    mods: u32,
    mode: u8,
    beatmap_id: i32,
    ranked_score: i64,
    accuracy: f32,
    playcount: i32,
    total_score: i64,
    rank: i32,
    pp: i16,
) -> Vec<u8> {
    packet!(
        PacketId::BANCHO_USER_STATS,
        data!(
            user_id,
            action,
            info,
            beatmap_md5,
            mods,
            mode,
            beatmap_id,
            ranked_score,
            accuracy / 100f32,
            playcount,
            total_score,
            rank,
            pp
        )
    )
}

```

**Available attributies**

- `ReadPacket`: This derive macro will implement the `BanchoPacketRead` trait for the struct.
- `WritePacket`: This derive macro will implement the `BanchoPacketRead` trait for the struct.
- `PacketLength`: This derive macro will implement the `BanchoPacketLength` trait for the struct.

example

```rust
use bancho_packets::{ReadPacket, PacketReader, PayloadReader};

#[derive(Debug, Clone, ReadPacket)]
/// [`BanchoMessage`] is the message structure of the bancho client.
pub struct BanchoMessage {
    pub sender: String,
    pub content: String,
    pub target: String,
    pub sender_id: i32,
}

// Now we can use [`PayloadReader`] to read the [`BanchoMessage`] from bytes.
let mut reader = PacketReader::new(&[
    1, 0, 0, 20, 0, 0, 0, 11, 0, 11, 6, 228, 189, 160, 229, 165, 189,
    11, 4, 35, 111, 115, 117, 0, 0, 0, 0,
]);
let packet = reader.next().unwrap();

let mut payload_reader = PayloadReader::new(packet.payload.unwrap());
let message = payload_reader.read::<BanchoMessage>();

println!("{:?}: {:?}", packet.id, message);
```

### Run Test

```
cargo test
```

### Run Benchmark

```
cargo bench
```

