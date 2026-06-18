use fugue_core::ir::{Address, SegmentProperties};
use fugue_core::project::Project;

use crate::error::QuokkaBuilderError;
use crate::export::metadata::MetadataExporter;
use crate::proto::{self, quokka};

#[derive(Debug, Clone)]
pub(crate) struct SegmentIndex {
    segments: Vec<SegmentInfo>,
}

impl SegmentIndex {
    pub(crate) fn new(segments: Vec<SegmentInfo>) -> Self {
        Self { segments }
    }

    pub(crate) fn resolve(
        &self,
        project: &Project,
        address: Address,
        missing: MissingAddressPolicy,
    ) -> Result<ResolvedAddress, QuokkaBuilderError> {
        let Some(segment) = self
            .segments
            .iter()
            .find(|segment| segment.contains(address))
        else {
            return Ok(ResolvedAddress::missing(missing));
        };

        let segment_offset = u32::try_from(address.offset().saturating_sub(segment.start))
            .map_err(|source| QuokkaBuilderError::numeric("segment offset", source))?;
        let file_offset = project
            .segments()
            .resolve_to_offset(address)
            .and_then(|(_, offset)| i64::try_from(offset).ok())
            .unwrap_or(-1);

        Ok(ResolvedAddress {
            segment_index: segment.index,
            segment_offset,
            file_offset,
        })
    }

    pub(crate) fn sort_key(&self, address: Address) -> (u32, u64) {
        self.segments
            .iter()
            .find(|segment| segment.contains(address))
            .map(|segment| {
                (
                    segment.index,
                    address.offset().saturating_sub(segment.start),
                )
            })
            .unwrap_or((u32::MAX, u64::MAX))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SegmentInfo {
    index: u32,
    space: usize,
    start: u64,
    end: u64,
}

impl SegmentInfo {
    fn contains(&self, address: Address) -> bool {
        self.space == address.space().index()
            && address.offset() >= self.start
            && address.offset() < self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MissingAddressPolicy {
    Zero,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedAddress {
    segment_index: u32,
    segment_offset: u32,
    file_offset: i64,
}

impl ResolvedAddress {
    pub(crate) fn segment_index(&self) -> u32 {
        self.segment_index
    }

    pub(crate) fn segment_offset(&self) -> u32 {
        self.segment_offset
    }

    pub(crate) fn file_offset(&self) -> i64 {
        self.file_offset
    }

    fn missing(policy: MissingAddressPolicy) -> Self {
        match policy {
            MissingAddressPolicy::Zero => Self {
                segment_index: 0,
                segment_offset: 0,
                file_offset: -1,
            },
        }
    }
}

pub(crate) struct SegmentExporter;

impl SegmentExporter {
    pub(crate) fn populate(
        project: &Project,
        quokka: &mut proto::Quokka,
    ) -> Result<SegmentIndex, QuokkaBuilderError> {
        let mut views = Vec::new();

        for space in project.segments().spaces() {
            for view in project.segments().iter_views(space.id())? {
                views.push(view);
            }
        }

        views.sort_by_key(|view| (view.space().index(), view.start().offset()));

        let mut segments = Vec::with_capacity(views.len());
        for view in views {
            let index = u32::try_from(segments.len())
                .map_err(|source| QuokkaBuilderError::numeric("segment index", source))?;
            let size = u64::try_from(view.size())
                .map_err(|source| QuokkaBuilderError::numeric("segment size", source))?;
            let file_offset = project
                .segments()
                .resolve_to_offset(view.start())
                .and_then(|(_, offset)| i64::try_from(offset).ok())
                .unwrap_or(-1);

            segments.push(SegmentInfo {
                index,
                space: view.space().index(),
                start: view.start().offset(),
                end: view.start().offset().saturating_add(size),
            });

            quokka.segments.push(quokka::Segment {
                name: view.name().to_owned(),
                virtual_addr: view.start().offset(),
                size,
                permissions: Self::permissions(view.properties()),
                r#type: Self::segment_type(view.properties()) as i32,
                address_size: MetadataExporter::address_size(project) as i32,
                file_offset,
            });
        }

        Ok(SegmentIndex::new(segments))
    }

    fn permissions(properties: SegmentProperties) -> u32 {
        let mut permissions = 0;

        if properties.is_readable() {
            permissions |= 4;
        }
        if properties.is_writable() {
            permissions |= 2;
        }
        if properties.is_executable() {
            permissions |= 1;
        }

        permissions
    }

    fn segment_type(properties: SegmentProperties) -> quokka::segment::Type {
        if properties.is_executable() {
            quokka::segment::Type::SegmentCode
        } else if properties.is_uninitialised() {
            quokka::segment::Type::SegmentBss
        } else if properties.is_external() {
            quokka::segment::Type::SegmentExtern
        } else {
            quokka::segment::Type::SegmentData
        }
    }
}
