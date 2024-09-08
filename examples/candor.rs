use candor::{stats::Bucket, stats::Stats, Packet, Source};

use clap::Parser;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, Paragraph},
    DefaultTerminal, Frame,
};

const CHANNEL_COLORS: [Color; 10] = [
    Color::Blue,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Cyan,
    Color::LightBlue,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightCyan,
];

fn main() -> std::io::Result<()> {
    let mut app = App::new()?;
    let terminal = ratatui::init();
    let result = app.run(terminal);

    ratatui::restore();

    result
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// CAN adapter(s)
    adapter: Vec<String>,

    /// Bit rate for Virtual CAN interfaces
    #[arg(short, long, default_value = "125000")]
    baud: u32,

    /// Don't use colors for CAN interfaces
    #[arg(short, long)]
    no_color: bool,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

struct Channel {
    source: Source,
    stats: Stats,
}

struct App {
    cli: Cli,
    events: mpsc::Receiver<Packet>,
    channels: Vec<Channel>,
    packets: VecDeque<Packet>,
    buckets: VecDeque<Bucket>,
    ids: HashMap<u32, usize>, // CAN ID -> bucket index
    stats: Stats,
    selection: i32,
    idle: bool,
    show_adapter: bool,
    show_dlc: bool,
}

impl App {
    fn new() -> std::io::Result<Self> {
        let cli = Cli::parse();

        // attach packet channel to all adapters
        let (tx, rx) = mpsc::channel::<Packet>();
        let mut channels: Vec<Channel> = vec![];
        for iface in cli.adapter.iter() {
            let index = channels.len();
            let source = Source::new(iface, index, cli.baud, tx.clone())?;
            let channel = Channel {
                source,
                stats: Stats::default(),
            };
            channels.push(channel);
        }

        let show_adapter = cli.no_color && channels.len() > 1;

        Ok(Self {
            cli,
            events: rx,
            channels,
            packets: VecDeque::<Packet>::new(),
            buckets: VecDeque::default(),
            ids: HashMap::default(),
            stats: Stats::default(),
            selection: -1,
            idle: false,
            show_adapter,
            show_dlc: true,
        })
    }

    fn run(&mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
        let mut stop = false;
        let mut draw_time: Instant = Instant::now();
        let mut stats_time: Instant = Instant::now();
        let interval = Duration::from_secs(1);

        loop {
            let now = Instant::now();
            if now - stats_time > interval {
                for channel in self.channels.iter_mut() {
                    channel
                        .stats
                        .periodic(&mut self.buckets, channel.source.baud());
                }
                self.stats.periodic(&mut self.buckets, 1);
                stats_time = now;
            }
            if !stop && (!self.idle || (now - draw_time > interval)) {
                self.draw_frame(&mut terminal)?;
                draw_time = now;
                self.idle = true;
            }

            // update stats for any received packets
            while (Instant::now() - now) < Duration::from_millis(10) {
                match self.events.try_recv() {
                    Ok(packet) => {
                        let channel = self
                            .channels
                            .get_mut(packet.source)
                            .expect("channel for id");

                        let index =
                            *self.ids.entry(packet.id).or_insert_with(|| {
                                self.buckets.push_back(Bucket::new(
                                    packet.id,
                                    packet.extended,
                                ));
                                self.buckets.len() - 1
                            });

                        let bucket =
                            self.buckets.get_mut(index).expect("index for id");

                        channel.stats.packet(bucket, &packet);

                        //                        self.stats.packet(&packet);

                        self.packets.push_front(packet);
                        if self.packets.len() > 100 {
                            let removed = self.packets.pop_back();
                        }
                        self.idle = false;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // TODO: note the error, data stream is broken so may as well exit?
                        break;
                    }
                }
                if self.idle {
                    break;
                }
            }

            // service user input
            if event::poll(Duration::from_millis(5))? {
                self.idle = false;
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('s') => stop = !stop,
                        KeyCode::Char('A') => {
                            self.show_adapter = !self.show_adapter;
                        }
                        KeyCode::Char('D') => {
                            self.show_dlc = !self.show_dlc;
                        }
                        KeyCode::Up => self.move_selection(-1),
                        KeyCode::Down => self.move_selection(1),
                        _ => {} // TODO: show help etc.
                    }
                }
            }
        }
        Ok(())
    }

    fn channel_color(&self, index: usize) -> Color {
        if self.cli.no_color {
            Color::White
        } else {
            CHANNEL_COLORS[index]
        }
    }

    fn move_selection(&mut self, by: i32) {
        self.selection =
            (self.selection + by).clamp(0, self.buckets.len() as i32);
        self.idle = false;
    }

    fn draw_dump(&mut self, frame: &mut Frame, area: Rect) {
        let height = area.height;
        let mut lines: Vec<Line> = Vec::with_capacity(height as usize + 2);
        let count = self.channels.len();

        for packet in self.packets.iter() {
            let channel = self
                .channels
                .get_mut(packet.source)
                .expect("channel for source");

            let mut text = "".to_string();

            if self.show_adapter {
                text.push_str(format!("{:8}", channel.source.name()).as_str());
            }

            if packet.extended {
                text.push_str(format!("{:8X} ", packet.id).as_str());
            } else {
                text.push_str(format!("     {:3X} ", packet.id).as_str());
            }
            if self.show_dlc {
                text.push_str(format!("  [{}]  ", packet.bytes.len()).as_str());
            }

            for byte in packet.bytes.iter() {
                text.push_str(format!(" {:02x}", byte).as_str());
            }
            lines.push(
                Line::from(text)
                    .style(Style::new().fg(self.channel_color(packet.source))),
            );
        }
        let summary = Paragraph::new(lines)
            .block(Block::bordered().title(" Dump (A=adapter, D=DLC) "));
        frame.render_widget(summary, area);
    }

    fn draw_monitor(&mut self, frame: &mut Frame, area: Rect) {
        if self.buckets.is_empty() {
            let data = Paragraph::new("(No Data)")
                .block(Block::bordered().title(" Data "))
                .centered();
            frame.render_widget(data, area);
            return;
        }

        let height = area.height;
        let mut lines: Vec<Line> = Vec::with_capacity(height as usize + 2);
        for (index, info) in self.buckets.iter().enumerate() {
            let selected = self.selection as usize == index;

            // selection marker
            let mut text = if selected {
                "→ ".to_string()
            } else {
                "  ".to_string()
            };

            let color = self.channel_color(info.current.source);
            let channel = self
                .channels
                .get_mut(info.current.source)
                .expect("channel for id");
            let normal = if !selected {
                Style::default().fg(color)
            } else {
                Style::new().fg(color).bg(Color::White)
            };

            if self.show_adapter {
                text.push_str(
                    format!("{:8}  ", channel.source.name()).as_str(),
                );
            }

            text.push_str(format!("{:7}  ", info.count).as_str());

            if info.extended {
                text.push_str(format!("{:08X} ", info.id).as_str());
            } else {
                text.push_str(format!("     {:03X} ", info.id).as_str());
            }

            if info.missing.is_zero() {
                text.push_str(format!(" @ {:5.0?}", info.delta).as_str());
            } else {
                text.push_str(format!(" @ -{:5.0?}", info.missing).as_str());
            }

            text.push_str("  ");
            for byte in info.current.bytes.iter() {
                text.push_str(format!(" {:02x}", byte).as_str());
            }

            lines.push(Line::from(text).style(normal));

            if index > height as usize {
                break;
            }
        }
        let widget =
            Paragraph::new(lines).block(Block::bordered().title(" Monitor "));
        frame.render_widget(widget, area);
    }

    fn draw_frame(
        &mut self,
        terminal: &mut DefaultTerminal,
    ) -> std::io::Result<()> {
        terminal.draw(|frame| {
            let area = frame.area();

            // top line
            let color = if self.cli.no_color {
                Color::White
            } else {
                Color::Green
            };
            let title = Line::from(vec![
                Span::styled("CANdor ", Style::default().fg(color)),
                Span::styled(
                    env!("CARGO_PKG_VERSION"),
                    Style::default().fg(color),
                ),
            ]);
            frame.render_widget(&title, area);
            let hints = Line::from(vec![Span::styled(
                "(? for help, q to quit) ",
                Style::default().fg(Color::Gray),
            )])
            .alignment(Alignment::Right);
            frame.render_widget(&hints, area);

            let area = area.inner(Margin::new(0, 1));
            let constraints =
                vec![Constraint::Percentage(60), Constraint::Percentage(40)];
            let cols = Layout::horizontal(constraints).split(area);

            // main monitor panel
            self.draw_monitor(frame, cols[0]);

            // interfaces & summary
            let mut r: Vec<Constraint> = self
                .channels
                .iter()
                .map(|_| Constraint::Length(5))
                .collect();
            r.push(Constraint::Fill(1));
            let rows = Layout::vertical(&r).split(cols[1]);

            for (row, channel) in self.channels.iter().enumerate() {
                let stat = &channel.stats;
                let area = rows[row];
                let block = Block::bordered()
                    .border_style(Style::new().fg(self.channel_color(row)))
                    .title(format!(
                        " {} @ {}bps ",
                        channel.source.name(),
                        channel.source.baud(),
                    ));
                let inner = block.inner(area);
                frame.render_widget(block, area);

                let bar_area = Rect::new(inner.x, inner.y, inner.width, 1u16);
                let percent = stat.load.clamp(0, 100) as u16;
                let title = format!("{}% ({} pps)", percent, channel.stats.pps);
                let gauge = Gauge::default()
                    .style(Style::default().fg(self.channel_color(row)))
                    .label(title)
                    .percent(percent);
                frame.render_widget(gauge, bar_area);

                let text_area = Rect::new(
                    inner.x,
                    inner.y + 1,
                    inner.width,
                    inner.height - 1,
                );
                let text = format!("{} packets", stat.data);
                let load = Paragraph::new(text);
                frame.render_widget(load, text_area);
            }

            // stream dump
            self.draw_dump(frame, rows[r.len() - 1]);
        })?;

        Ok(())
    }
}
