// Copyright 2021 The Engula Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use futures::{future, stream};
use tokio::{
    fs,
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    sync::Mutex,
};

use crate::{
    async_trait,
    file::segment::{Index, Segment},
    Error, Event, Result, ResultStream, Timestamp,
};

const RELEASE_TS_FILE_NAME: &str = "release_timestamp";

#[derive(Clone)]
pub struct Stream {
    root: PathBuf,
    release_ts_file_path: PathBuf,
    index: Arc<Mutex<VecDeque<Index>>>,
    segments: Arc<Mutex<VecDeque<Segment>>>,
}

impl Stream {
    pub async fn create(root: PathBuf) -> Result<Stream> {
        let path = root.clone();
        fs::DirBuilder::new().recursive(true).create(&path).await?;
        match fs::DirBuilder::new().recursive(true).create(&path).await {
            Ok(_) => {
                let segments = Arc::new(Mutex::new(VecDeque::new()));
                let index = Arc::new(Mutex::new(VecDeque::new()));
                let release_ts_file_path = Stream::release_ts_file_path(path.clone());
                let mut stream = Stream {
                    root: path,
                    release_ts_file_path,
                    index,
                    segments,
                };
                stream.try_recovery().await?;
                Ok(stream)
            }
            Err(e) => Err(Error::Unknown(e.to_string())),
        }
    }

    fn segment_path(&self, index: usize) -> PathBuf {
        // file name format: fix ten size 000-{binary value of index}
        // such as 0000000001 for 1 and 0000000011 for 2
        self.root.join(format!("{:010b}", index))
    }

    fn release_ts_file_path(dir: PathBuf) -> PathBuf {
        dir.join(RELEASE_TS_FILE_NAME)
    }

    pub async fn clean(&self) -> Result<()> {
        fs::remove_dir_all(self.root.clone()).await?;
        Ok(())
    }

    async fn read_events_internal(&self, ts: Timestamp) -> Result<ResultStream<Vec<Event>>> {
        let indexes = self.index.lock().await;
        let mut segments = self.segments.lock().await;

        let offset = indexes.partition_point(|x| x.ts < ts);

        let index_option = indexes.get(offset);

        if index_option.is_none() {
            return Ok(Box::new(stream::once(future::ok(Vec::new()))));
        }

        let index = index_option.unwrap();

        let mut events = Vec::new();

        for i in (0..segments.len()).rev() {
            let segment = &mut segments[i];
            let start_index = segment.start_index.as_ref().unwrap();
            let end_index = segment.end_index.as_ref().unwrap();

            if end_index.ts < index.ts {
                continue;
            }

            let start_location = if start_index.ts < index.ts {
                index.location
            } else {
                start_index.location
            };
            let end_location = end_index.location + end_index.size;

            events.append(segment.read(start_location, end_location).await?.as_mut());
        }
        Ok(Box::new(stream::once(future::ok(events))))
    }

    async fn try_recovery(&mut self) -> Result<()> {
        let mut delete_time = None;

        let delete_file_result = File::open(&self.release_ts_file_path).await;
        if let Ok(deleted_file) = delete_file_result {
            let mut delete_reader = BufReader::new(deleted_file);
            delete_time = Some(delete_reader.read_u64().await?);
        }

        if let Ok(segment_file) = Stream::get_segment_file(self.root.clone()).await {
            let mut segments = self.segments.lock().await;
            let mut indexes = self.index.lock().await;

            for segment_file in segment_file {
                let mut segment = Segment::create(segment_file).await?;

                for index in segment.read_index(0, segment.position).await? {
                    if let Some(delete) = delete_time {
                        if index.ts < Timestamp::from(delete) {
                            continue;
                        }
                    }
                    indexes.push_back(index);
                }

                if let Some(old) = segments.get_mut(0) {
                    old.become_read_only();
                }
                segments.push_front(segment);
            }
        }
        Ok(())
    }

    async fn get_segment_file(root: impl Into<PathBuf>) -> Result<Vec<PathBuf>> {
        let path = root.into();

        let mut stream_list: Vec<PathBuf> = Vec::new();

        let mut dir = fs::read_dir(path).await?;
        while let Some(child) = dir.next_entry().await? {
            if child.metadata().await?.is_file() {
                let name = child.file_name();
                if name.ne(RELEASE_TS_FILE_NAME) {
                    stream_list.push(child.path())
                }
            }
        }
        // with file name format in segment_path function, it will sort in right way
        stream_list.sort();
        Ok(stream_list)
    }
}

#[async_trait]
impl crate::Stream for Stream {
    async fn read_events(&self, ts: Timestamp) -> ResultStream<Vec<Event>> {
        let output = self.read_events_internal(ts).await;
        match output {
            Ok(output) => output,
            Err(e) => Box::new(futures::stream::once(futures::future::err(e))),
        }
    }

    async fn append_event(&self, event: Event) -> Result<()> {
        let mut indexes = self.index.lock().await;
        let mut segments = self.segments.lock().await;

        if segments.is_empty() || segments.front().unwrap().is_full() {
            let segment_path: PathBuf = self.segment_path(segments.len() + 1);
            let segment = Segment::create(segment_path).await?;
            if let Some(old) = segments.get_mut(0) {
                old.become_read_only();
            }
            segments.push_front(segment);
        }

        let active_segment = segments.front_mut().unwrap();
        let index = active_segment.write(event).await?;
        indexes.push_back(index);

        Ok(())
    }

    async fn release_events(&self, ts: Timestamp) -> Result<()> {
        let mut indexes = self.index.lock().await;
        let mut segments = self.segments.lock().await;

        let offset = indexes.partition_point(|x| x.ts < ts);

        for i in (0..segments.len()).rev() {
            let segment = &mut segments[i];
            let end_index = segment.end_index.as_ref().unwrap();
            if end_index.ts < ts {
                segment.clean().await?;
                segments.drain(i..i + 1);
            }
        }

        let mut time_buf = [0_u8; 8];
        let time_bytes = ts.serialize();
        time_buf.clone_from_slice(&time_bytes);

        // create will truncate old content
        let delete_file = File::create(&self.release_ts_file_path).await?;
        let mut delete_writer = BufWriter::new(delete_file);
        delete_writer.write(&time_buf).await?;
        delete_writer.flush().await?;

        indexes.drain(..offset);

        Ok(())
    }
}
