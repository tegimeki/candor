use candor::{
    sources::{peak_trace::PeakTraceSource, Source},
    stats::Stats,
    Packet,
};

#[cfg(feature = "socketcan")]
use candor::sources::socketcan::SocketCanSource;

use clap::Parser;
use regex::Regex;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cli_log::*;

use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Cell, Gauge, Paragraph, Row, Table, TableState},
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

fn main() -> Result<()> {
    init_cli_log!();

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

    /// Sync time across multiple trace files
    #[arg(short, long)]
    sync_time: bool,

    /// Don't use colors
    #[arg(short, long)]
    no_color: bool,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

struct Channel {
    source: Box<dyn Source>,
    stats: Stats,
}

struct App {
    cli: Cli,
    events: mpsc::Receiver<Packet>,
    channels: Vec<Channel>,
    packets: VecDeque<Packet>,
    table_state: TableState,
    width: u16,
    expanded: bool,
    order: usize,
    idle: bool,
    show_source: bool,
    enable_decode: bool,
    show_undecoded: bool,
    show_ascii: bool,
    show_bin: bool,
    visible_messages: u16,
}

impl App {
    fn new() -> Result<Self> {
        let cli = Cli::parse();

        // attach packet channel to all adapters
        let (tx, rx) = mpsc::channel::<Packet>();
        let mut channels: Vec<Channel> = vec![];
        for iface in cli.adapter.iter() {
            let index = channels.len();
            let (ifname, dbcs) = App::parse_source(iface);
            let path = Path::new(&ifname);
            let extension = match path.extension() {
                Some(s) => s.to_str().unwrap_or(""),
                None => "",
            };

            let source: Box<dyn Source> = match extension {
                "trc" => Box::new(PeakTraceSource::new(
                    ifname.as_str(),
                    index,
                    cli.baud,
                    cli.sync_time,
                    tx.clone(),
                )?),

                #[cfg(not(feature = "socketcan"))]
                _ => return Err(Error::from(ErrorKind::InvalidInput)),

                #[cfg(feature = "socketcan")]
                _ => Box::new(SocketCanSource::new(
                    ifname.as_str(),
                    index,
                    cli.baud,
                    tx.clone(),
                )?),
            };

            let baud = source.baud();
            let mut channel = Channel {
                source,
                stats: Stats::new(baud),
            };
            for dbc in dbcs {
                channel.stats.add_dbc(dbc)?;
            }
            channels.push(channel);
        }

        let show_source = cli.no_color && channels.len() > 1;

        Ok(Self {
            cli,
            events: rx,
            channels,
            packets: VecDeque::<Packet>::new(),
            table_state: TableState::default().with_selected(0),
            width: 60,
            expanded: true,
            order: 0,
            idle: false,
            show_source,
            enable_decode: true,
            show_undecoded: true,
            show_ascii: false,
            show_bin: false,
            visible_messages: 1,
        })
    }

    fn run(&mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
        let mut stop = false;
        let mut draw_time: Instant = Instant::now();
        let mut stats_time: Instant = Instant::now();
        let interval = Duration::from_secs(1);

        loop {
            let now = Instant::now();
            if now - stats_time >= interval {
                for channel in self.channels.iter_mut() {
                    channel.stats.periodic();
                }
                stats_time = now;
            }

            if !stop && (!self.idle || (now - draw_time > interval)) {
                terminal.draw(|frame| self.draw(frame))?;
                draw_time = now;
                self.idle = true;
            }

            // update stats for received packets
            while (Instant::now() - now) < Duration::from_millis(10) {
                match self.events.try_recv() {
                    Ok(packet) => {
                        let channel = self
                            .channels
                            .get_mut(packet.source)
                            .expect("channel for id");

                        channel.stats.packet(&packet);

                        self.packets.push_front(packet);
                        if self.packets.len() > 100 {
                            let _ = self.packets.pop_back();
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
                        KeyCode::Esc => stop = !stop,
                        KeyCode::Char('q') => break,
                        KeyCode::Char('S') => {
                            self.show_source = !self.show_source;
                        }
                        KeyCode::Char('A') => {
                            self.show_ascii = !self.show_ascii;
                            self.show_bin = false;
                        }
                        KeyCode::Char('B') => {
                            self.show_bin = !self.show_bin;
                            self.show_ascii = false;
                        }
                        KeyCode::Char('d') => {
                            self.enable_decode = !self.enable_decode;
                        }
                        // width adjustment
                        KeyCode::Char('W') => {
                            self.width += 1;
                            self.width = self.width.clamp(30, 70);
                        }
                        KeyCode::Char('w') => {
                            self.width -= 1;
                            self.width = self.width.clamp(30, 70);
                        }
                        // show/hide undecoded messages
                        KeyCode::Char('u') => {
                            self.show_undecoded = !self.show_undecoded;
                        }
                        // bus order
                        KeyCode::Char('<') => {
                            self.order = self.next_channel(self.order)
                        }
                        KeyCode::Char('>') => {
                            self.order = self.prev_channel(self.order)
                        }
                        KeyCode::Right => self.expand(),
                        KeyCode::Left => self.collapse(),
                        KeyCode::Up => self.update_selection(-1),
                        KeyCode::Down => self.update_selection(1),
                        KeyCode::PageUp => self
                            .update_selection(-(self.visible_messages as i32)),
                        KeyCode::PageDown => {
                            self.update_selection(self.visible_messages as i32)
                        }
                        _ => {} // TODO: show help etc.
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse <ifname>[:<filename.dbc>] specifier to allow associating
    /// DBC file(s) with a source interface
    fn parse_source(name: &str) -> (String, Vec<String>) {
        let mut dbcs: Vec<String> = vec![];

        let re = Regex::new(r"([^:]+)([:]*)(.*)").unwrap();
        let c = re.captures(name).unwrap();

        let ifname = c.get(1).unwrap().as_str().to_string();
        let sep = c.get(2).unwrap().as_str();

        if sep == ":" {
            let dbc = c.get(3).unwrap().as_str().to_string();
            dbcs.push(dbc);
        }
        (ifname, dbcs)
    }

    fn channel_color(&self, index: usize) -> Color {
        if self.cli.no_color {
            Color::White
        } else {
            CHANNEL_COLORS[index]
        }
    }

    fn max_selection(&self) -> usize {
        self.channels
            .iter()
            .map(|c| {
                c.stats
                    .messages()
                    .iter()
                    .filter(|m| self.show_undecoded || m.dbc.is_some())
                    .count()
            })
            .sum::<usize>()
    }

    fn expand(&mut self) {
        self.expanded = true;
    }

    fn collapse(&mut self) {
        self.expanded = false;
    }

    fn update_selection(&mut self, by: i32) {
        let current = self.table_state.selected().unwrap_or(0) as i32;
        let mut new = current + by;
        let max = self.max_selection() as i32;
        new = new.clamp(0, max - 1);
        self.table_state.select(Some(new as usize));
    }

    fn next_channel(&self, index: usize) -> usize {
        if index > 0 {
            index - 1
        } else {
            self.channels.len() - 1
        }
    }

    fn prev_channel(&self, index: usize) -> usize {
        if index < self.channels.len() - 1 {
            index + 1
        } else {
            0
        }
    }

    fn draw_dump(&mut self, frame: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }

        let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize + 2);
        let mut count = area.height;

        for packet in self.packets.iter() {
            let channel = self
                .channels
                .get_mut(packet.source)
                .expect("channel for source");

            let mut text = "".to_string();

            if self.show_source {
                text.push_str(format!("{:8}", channel.source.name()).as_str());
            }

            if packet.extended {
                text.push_str(format!("{:8X} ", packet.id).as_str());
            } else {
                text.push_str(format!("     {:3X} ", packet.id).as_str());
            }
            text.push_str(format!("  [{}]  ", packet.bytes.len()).as_str());

            for byte in packet.bytes.iter() {
                text.push_str(format!(" {:02x}", byte).as_str());
            }
            lines.push(
                Line::from(text)
                    .style(Style::new().fg(self.channel_color(packet.source))),
            );
            count -= 1;
            if count == 0 {
                break;
            }
        }
        let summary = Paragraph::new(lines)
            .block(Block::bordered().title(" Dump  (S=show source)"));
        frame.render_widget(summary, area);
    }

    fn draw_messages(&mut self, frame: &mut Frame, area: Rect) {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);

        let mut rows: Vec<Row> = Vec::with_capacity(area.height as usize + 2);
        let channel_count = self.channels.len();
        let mut order = self.order;
        for _ in 0..channel_count {
            let channel = self.channels.get(order).unwrap();

            let messages = channel.stats.messages();
            for message_index in channel.stats.ordering().iter() {
                let message = messages.get(*message_index).unwrap();
                if !self.show_undecoded && message.dbc.is_none() {
                    continue;
                }

                let color = self.channel_color(message.current.source);
                let row_style = Style::default().fg(color);

                let mut height = 1;

                let dbc_message = if self.enable_decode {
                    channel.stats.dbc_message(message)
                } else {
                    None
                };

                // Message name / ID
                let mut id = "".to_string();
                if let Some(msg) = dbc_message {
                    id.push_str(msg.message_name().as_str());
                    id.push('\n');
                    height += 1;
                }
                if message.extended {
                    id.push_str(format!("{:08X} ", message.id).as_str());
                } else {
                    id.push_str(format!("     {:03X} ", message.id).as_str());
                };

                // period
                let period = if message.missing.is_zero() {
                    let q = ((message.delta.as_millis() as u64 + 5) / 10) * 10;
                    format!("{:5.0?}", Duration::from_millis(q))
                } else {
                    format!("! -{:5.0?}", message.missing)
                };

                // raw data
                let mut data = "".to_string();
                if self.show_bin {
                    for byte in message.current.bytes.iter() {
                        data.push_str(format!("{:08b}", byte).as_str());
                    }
                } else {
                    for byte in message.current.bytes.iter() {
                        data.push_str(format!("{:02x} ", byte).as_str());
                    }
                    if self.show_ascii {
                        let len = message.current.bytes.len();
                        for _ in len..9 {
                            data.push_str("   ");
                        }
                        for byte in message.current.bytes.iter().rev() {
                            if *byte >= 0x20 && *byte <= 0x7F {
                                data.push(*byte as char);
                            } else {
                                data.push('.');
                            }
                        }
                    }
                }

                // signals
                if self.expanded {
                    if let Some(msg) = dbc_message {
                        for signal in msg.signals().iter() {
                            let value = channel.stats.signal_text(
                                msg,
                                signal,
                                &message.current,
                            );
                            if value.is_empty() {
                                continue;
                            }
                            let text =
                                format!("\n  {} {}", signal.name(), value);
                            data.push_str(text.as_str());
                            height += 1;
                        }
                    }
                }

                let row = Row::new(
                    vec![&id, &period, &data]
                        .into_iter()
                        .map(|s| Cell::from(Text::from(s.clone()))),
                )
                .height(height)
                .style(row_style);

                rows.push(row);
            }
            order = self.next_channel(order);
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(24),
                Constraint::Length(10),
                Constraint::Fill(1),
            ],
        )
        .highlight_style(selected_style)
        .block(Block::bordered().title(
            " Message──────────────── Period ─── Data (A=ASCII, B=binary) ",
        ));

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // top line
        let color = if self.cli.no_color {
            Color::White
        } else {
            Color::Green
        };
        let title = Line::from(vec![
            Span::bold(" ⚡︎ CANdor ".into()),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(color)),
        ]);
        frame.render_widget(&title, area);
        let hints = Line::from(vec![Span::styled(
            "(? for help, q to quit) ",
            Style::default().fg(Color::Gray),
        )])
        .alignment(Alignment::Right);
        frame.render_widget(&hints, area);

        let area = area.inner(Margin::new(0, 1));
        let constraints = vec![
            Constraint::Percentage(self.width),
            Constraint::Percentage(100 - self.width),
        ];
        let cols = Layout::horizontal(constraints).split(area);
        self.visible_messages = cols[0].height - 2;

        // main messages panel
        self.draw_messages(frame, cols[0]);

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

            let text_area =
                Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1);
            let message_count = stat.messages().len();
            let text = format!(
                "{} packets, {} unique by ID",
                stat.packets, message_count
            );
            let load = Paragraph::new(text);
            frame.render_widget(load, text_area);
        }

        // stream dump
        self.draw_dump(frame, rows[r.len() - 1]);
    }
}
