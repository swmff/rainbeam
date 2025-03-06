use serde::{Serialize, Deserialize};
use crate::{database::Result, model::DatabaseError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Point(
    usize,
    usize,
    #[serde(default, skip_serializing_if = "Option::is_none")] Option<String>,
);
pub type Line = Vec<Point>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageConfig {
    #[serde(alias = "w")]
    pub width: usize,
    #[serde(alias = "h")]
    pub height: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CarpGraph {
    #[serde(alias = "i")]
    pub image: ImageConfig,
    #[serde(alias = "d")]
    pub data: Vec<Line>,
}

impl CarpGraph {
    pub fn from_str(input: &str) -> Result<Self> {
        match serde_json::from_str(input) {
            Ok(de) => Ok(de),
            Err(_) => Err(DatabaseError::ValueError),
        }
    }

    pub fn to_svg(&self) -> String {
        let mut out: String = String::new();
        out.push_str(&format!(
            "<svg viewBox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\" style=\"background: white; width: {}px; height: {}px\" class=\"carpgraph\">",
            self.image.width, self.image.height, self.image.width, self.image.height
        ));

        // add lines
        let mut stroke_size: i8 = 1;
        let mut stroke_color: String = "#000000".to_string();

        for line in &self.data {
            let mut previous_x_y: Option<(usize, usize)> = None;
            let mut line_path = String::new();

            for point in line {
                // adjust brush color/size
                if let Some(ref color_or_size) = point.2 {
                    if color_or_size.starts_with("#") {
                        stroke_color = color_or_size.to_string();
                    } else {
                        stroke_size = color_or_size.parse::<i8>().unwrap();
                    }
                }

                // add to path string
                line_path.push_str(&format!(
                    " M{} {}{}",
                    point.0,
                    point.1,
                    if let Some(pxy) = previous_x_y {
                        // line to there
                        format!(" L{} {}", pxy.0, pxy.1)
                    } else {
                        String::new()
                    }
                ));

                previous_x_y = Some((point.0, point.1));

                // add circular point
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{stroke_color}\" />",
                    point.0,
                    point.1,
                    stroke_size / 2 // the size is technically the diameter of the circle
                ));
            }

            out.push_str(&format!(
                "<path d=\"{line_path}\" stroke=\"{stroke_color}\" stroke-width=\"{stroke_size}\" />"
            ));
        }

        // return
        format!("{out}</svg>")
    }
}
