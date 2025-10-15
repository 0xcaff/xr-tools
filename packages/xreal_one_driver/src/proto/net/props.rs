use crate::proto::net::Response;
use crate::proto::usb::RequestArgs;
use anyhow::bail;
use protobuf::rt::WireType;
use protobuf::CodedInputStream;
use std::borrow::Cow;
use std::error::Error;
use std::io::Cursor;

pub trait PropertyValue: Sized {
    const WIRE_TYPE: WireType;
}

pub trait PropertyValueWrite: PropertyValue {
    fn measure(&self) -> Result<u64, anyhow::Error>;
    fn write(&self, os: &mut protobuf::CodedOutputStream<'_>) -> Result<(), anyhow::Error>;
}

pub trait PropertyValueRead: PropertyValue {
    fn read(is: &mut protobuf::CodedInputStream<'_>) -> Result<Self, anyhow::Error>;
}

#[derive(Debug)]
pub struct SetPropertyRequest<V: PropertyValueWrite> {
    pub value: V,
}

impl<V: PropertyValueWrite> ProtobufWrite for SetPropertyRequest<V> {
    fn write(&self, os: &mut protobuf::CodedOutputStream<'_>) -> Result<(), anyhow::Error> {
        let length = self.value.measure()?;

        os.write_tag(3, WireType::LengthDelimited)?;
        os.write_raw_varint64(length + 1)?;

        os.write_tag(1, V::WIRE_TYPE)?;
        self.value.write(os)?;

        Ok(())
    }
}

impl<V: ProtobufWrite> RequestArgs<'static> for V {
    fn as_bytes(&self) -> Result<Cow<'static, [u8]>, anyhow::Error> {
        let mut bytes = Vec::<u8>::new();
        {
            let os = Cursor::new(&mut bytes);
            let mut boxed = Box::new(os);
            let mut output_stream = protobuf::CodedOutputStream::new(&mut boxed);

            self.write(&mut output_stream)?;

            output_stream.flush()?;
        }

        Ok(Cow::Owned(bytes))
    }
}

trait ProtobufWrite {
    fn write(&self, os: &mut protobuf::CodedOutputStream<'_>) -> Result<(), anyhow::Error>;
}

pub struct GetPropertyRequest;

impl ProtobufWrite for GetPropertyRequest {
    fn write(&self, os: &mut protobuf::CodedOutputStream<'_>) -> Result<(), anyhow::Error> {
        os.write_tag(3, WireType::Varint)?;
        os.write_raw_varint64(0)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct PropertyResponse<V: PropertyValueRead> {
    pub value: V,
}

impl<V: PropertyValueRead> ProtobufRead for PropertyResponse<V> {
    fn read(is: &mut protobuf::CodedInputStream<'_>) -> Result<Self, anyhow::Error> {
        let Some(tag) = is.read_raw_tag_or_eof()? else {
            bail!("unexpected end of stream");
        };

        if tag != ((4 << 4) | WireType::LengthDelimited as u32) {
            bail!("unexpected tag: {}", tag);
        }

        let len = is.read_raw_varint64()?;
        let Some(tag) = is.read_raw_tag_or_eof()? else {
            bail!("unexpected end of stream");
        };

        if tag != ((2 << 4) | V::WIRE_TYPE as u32) {
            bail!("unexpected tag: {}", tag);
        }

        let value = V::read(is)?;

        is.check_eof()?;

        Ok(Self { value })
    }
}

trait ProtobufRead: Sized {
    fn read(is: &mut protobuf::CodedInputStream<'_>) -> Result<Self, anyhow::Error>;
}

impl<V: ProtobufRead> Response for V {
    fn deserialize_from(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        let mut is = protobuf::CodedInputStream::from_bytes(&buffer);
        Self::read(&mut is)
    }
}

#[derive(Debug)]
pub struct EmptyPropertyResponse;

impl ProtobufRead for EmptyPropertyResponse {
    fn read(is: &mut protobuf::CodedInputStream<'_>) -> Result<Self, anyhow::Error> {
        let Some(tag) = is.read_raw_tag_or_eof()? else {
            bail!("unexpected end of stream");
        };

        if tag != ((4 << 3) | WireType::LengthDelimited as u32) {
            bail!("unexpected tag: {:x}", tag);
        }

        let len = is.read_raw_varint64()?;
        if len != 0 {
            bail!("unexpected length: {}", len);
        }

        is.check_eof()?;
        Ok(Self)
    }
}

impl<T> PropertyValue for SetNumericProperty<T>
where
    T: Into<u8>,
{
    const WIRE_TYPE: WireType = WireType::Varint;
}

impl<T> PropertyValueWrite for SetNumericProperty<T>
where
    T: Into<u8> + Copy,
{
    fn measure(&self) -> Result<u64, anyhow::Error> {
        Ok(protobuf::rt::compute_raw_varint64_size(
            Into::<u8>::into(self.0) as u64,
        ))
    }

    fn write(&self, os: &mut protobuf::CodedOutputStream<'_>) -> Result<(), anyhow::Error> {
        os.write_raw_varint32(Into::<u8>::into(self.0) as u32)?;
        Ok(())
    }
}

pub struct ReadNumericProperty<T>(pub T);

impl<T> PropertyValue for ReadNumericProperty<T>
where
    T: TryFrom<u8>,
{
    const WIRE_TYPE: WireType = WireType::Varint;
}

impl<T> PropertyValueRead for ReadNumericProperty<T>
where
    T: TryFrom<u8>,
    T::Error: Send + Sync + Error + 'static,
{
    fn read(is: &mut protobuf::CodedInputStream<'_>) -> Result<Self, anyhow::Error> {
        Ok(Self(T::try_from(is.read_raw_varint32()? as u8)?))
    }
}

pub struct SetNumericProperty<T>(pub T);

impl PropertyValue for String {
    const WIRE_TYPE: WireType = WireType::LengthDelimited;
}

impl PropertyValueRead for String {
    fn read(is: &mut CodedInputStream<'_>) -> Result<Self, anyhow::Error> {
        Ok(is.read_string()?)
    }
}
