use crate::*;


pub type FileServerHandle = u16;


pub trait FileServer
{
	fn get_handle(
		&mut self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		filename: &str)
		-> Result<FileServerHandle, ()>;


	fn get_handle_unwrap(
		&mut self,
		filename: &str)
		-> FileServerHandle
	{
		self.get_handle(
				&mut diagn::Report::new(),
				None,
				filename)
			.unwrap()
	}


	fn get_filename(
		&self,
		file_handle: FileServerHandle)
		-> &str;

	
	fn get_bytes(
		&self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		file_handle: FileServerHandle)
		-> Result<Vec<u8>, ()>;
	
	
	fn get_str(
		&self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		file_handle: FileServerHandle)
		-> Result<String, ()>
	{
		let bytes = self.get_bytes(
			report,
			span,
			file_handle)?;

		let string = String::from_utf8_lossy(&bytes).to_string();
		
		Ok(string)
	}
	
	
	fn get_str_unwrap(
		&self,
		file_handle: FileServerHandle)
		-> String
	{
		self.get_str(
				&mut diagn::Report::new(),
				None,
				file_handle)
			.unwrap()
	}
	
	
	fn write_bytes(
		&mut self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		filename: &str,
		data: &Vec<u8>)
		-> Result<(), ()>;
	
	
	fn get_excerpt(
		&self,
		span: diagn::Span)
		-> String
	{
		if let Ok(chars) = self.get_str(
			&mut diagn::Report::new(),
			None,
			span.file_handle)
		{
			let counter = util::CharCounter::new(&chars);
			let location = span.location().unwrap();
			counter.get_excerpt(location.0, location.1).iter().collect()
		}
		else
		{
			"".to_string()
		}
	}
}


pub struct FileServerMock
{
	handles: std::collections::HashMap<String, FileServerHandle>,
	handles_to_filename: Vec<String>,
	files: Vec<Vec<u8>>,
}


pub struct FileServerReal
{
	handles: std::collections::HashMap<String, FileServerHandle>,
	handles_to_filename: Vec<String>,
}


impl FileServerMock
{
	pub fn new() -> FileServerMock
	{
		FileServerMock {
			handles: std::collections::HashMap::new(),
			handles_to_filename: Vec::new(),
			files: Vec::new(),
		}
	}
	
	
	pub fn add<S, T>(
		&mut self,
		filename: S,
		contents: T)
		where S: Into<String>, T: Into<Vec<u8>>
	{
		let filename = filename.into();

		let next_index = self.handles.len();

		let handle = *self.handles
			.entry(filename.clone())
			.or_insert(next_index.try_into().unwrap());

		while handle as usize >= self.files.len()
		{
			self.handles_to_filename.push("".to_string());
			self.files.push(Vec::new());
		}

		self.handles_to_filename[handle as usize] = filename;
		self.files[handle as usize] = contents.into();
	}
}


impl FileServerReal
{
	pub fn new() -> FileServerReal
	{
		FileServerReal {
			handles: std::collections::HashMap::new(),
			handles_to_filename: Vec::new(),
		}
	}
}


impl FileServer for FileServerMock
{
	fn get_handle(
		&mut self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		filename: &str)
		-> Result<FileServerHandle, ()>
	{
		if self.handles.len() == FileServerHandle::MAX as usize
		{
			report_error(
				report,
				span,
				"exhausted number of file handles");

			return Err(());
		}

		if !self.handles.contains_key(filename)
		{
			report_error(
				report,
				span,
				format!(
					"file not found: `{}`",
					filename));

			return Err(());
		}

		let handle = self.handles.get(filename).unwrap();

		Ok(*handle)
	}


	fn get_filename(
		&self,
		file_handle: FileServerHandle)
		-> &str
	{
		&self.handles_to_filename[file_handle as usize]
	}


	fn get_bytes(
		&self,
		_report: &mut diagn::Report,
		_span: Option<diagn::Span>,
		file_handle: FileServerHandle)
		-> Result<Vec<u8>, ()>
	{
		Ok(self.files[file_handle as usize].clone())
	}
	
	
	fn write_bytes(
		&mut self,
		_report: &mut diagn::Report,
		_span: Option<diagn::Span>,
		filename: &str,
		data: &Vec<u8>)
		-> Result<(), ()>
	{
		let new_index = self.handles.len();

		let handle = *self.handles
			.entry(filename.into())
			.or_insert(new_index.try_into().unwrap());

		while handle as usize >= self.files.len()
		{
			self.files.push(Vec::new());
		}

		self.files[handle as usize] = data.clone();
		
		Ok(())
	}
}


impl FileServer for FileServerReal
{
	fn get_handle(
		&mut self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		filename: &str)
		-> Result<FileServerHandle, ()>
	{
		let filename_path = &std::path::Path::new(filename);
		
		if !filename_path.exists()
		{
			report_error(
				report,
				span,
				format!(
					"file not found: `{}`",
					filename));
			
			return Err(());
		}
		
		if self.handles.len() == FileServerHandle::MAX as usize
		{
			report_error(
				report,
				span,
				"exhausted number of file handles");

			return Err(());
		}

		match self.handles.get(filename)
		{
			Some(handle) => Ok(*handle),
			None =>
			{
				let handle =
					self.handles.len() as FileServerHandle;

				self.handles.insert(
					filename.to_string(),
					handle);

				self.handles_to_filename.push(
					filename.to_string());

				Ok(handle)
			}
		}
	}


	fn get_filename(
		&self,
		file_handle: FileServerHandle)
		-> &str
	{
		&self.handles_to_filename[file_handle as usize]
	}


	fn get_bytes(
		&self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		file_handle: FileServerHandle)
		-> Result<Vec<u8>, ()>
	{
		let filename = &self.handles_to_filename[file_handle as usize];
		let filename_path = &std::path::Path::new(filename);
		
		let mut file = {
			match std::fs::File::open(filename_path)
			{
				Ok(file) => file,
				Err(err) =>
				{
					report_error(
						report,
						span,
						format!(
							"could not open file `{}`: {}",
							filename,
							err));
					
					return Err(());
				}
			}
		};

		let mut vec = Vec::new();

		use std::io::Read;
		match file.read_to_end(&mut vec)
		{
			Ok(_) => Ok(vec),
			Err(err) =>
			{
				report_error(
					report,
					span,
					format!(
						"could not read file `{}`: {}",
						filename,
						err));
				
				return Err(());
			}
		}
	}
	
	
	fn write_bytes(
		&mut self,
		report: &mut diagn::Report,
		span: Option<diagn::Span>,
		filename: &str,
		data: &Vec<u8>)
		-> Result<(), ()>
	{
		let filename_path = &std::path::Path::new(filename);
		
		let mut file = {
			match std::fs::File::create(filename_path)
			{
				Ok(file) => file,
				Err(err) =>
				{
					report_error(
						report,
						span,
						format!(
							"could not create file `{}`: {}",
							filename,
							err));

					return Err(());
				}
			}
		};

		use std::io::Write;
		match file.write_all(data)
		{
			Ok(_) => Ok(()),
			Err(err) => 
			{
				report_error(
					report,
					span,
					format!("could not write to file `{}`: {}",
						filename,
						err));

				Err(())
			}
		}
	}
}


fn report_error<S>(
	report: &mut diagn::Report,
	span: Option<diagn::Span>,
	descr: S)
	where S: Into<String>
{
	if let Some(span) = span
	{
		report.error_span(descr, span);
	}
	else
	{
		report.error(descr);
	}
}