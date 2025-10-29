use egui::{Color32, Pos2, Slider, Vec2};
// eframe TemplateApp 구조체
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    center: Vec2,
    radius: f32,
    color: Color32,
}

// 상수
mod consts {
    use egui::Color32;

    // 해상도
    pub const MAX_RESOLUTION_X: f32 = 1280.0;
    pub const MAX_RESOLUTION_Y: f32 = 720.0;

    // 월드 좌표계 기준 기본값
    pub const DEFAULT_CENTER_X: f32 = 0.0;
    pub const DEFAULT_CENTER_Y: f32 = 0.0;
    pub const DEFAULT_RADIUS: f32 = 0.3;
    pub const MAX_RADIUS: f32 = 1.0;
    pub const DEFAULT_COLOR: Color32 = Color32::RED;
    pub const DEFAULT_BACKGROUND_COLOR: Color32 = Color32::GRAY;
}

// 앱이 처음 켜질 때의 기본값 (월드 좌표계 기준)
impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            center: Vec2::new(
                consts::DEFAULT_CENTER_X,
                consts::DEFAULT_CENTER_Y,
            ),
            radius: consts::DEFAULT_RADIUS,
            color: consts::DEFAULT_COLOR,
        }
    }
}

// TemplateApp 생성자 및 헬퍼 함수
impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 상태 저장
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY)
        //         .unwrap_or_default();
        // }
        Self::default()
    }

    // 스크린 좌표계 [0, width-1] x [0, height-1]를
    // 월드 좌표계 [-aspect, +aspect] x [-1, +1]로 변환 (Y축 반전 포함)
    fn transform_screen_to_world(
        &self,
        position_screen: Pos2,
    ) -> Vec2 {
        let width = consts::MAX_RESOLUTION_X;
        let height = consts::MAX_RESOLUTION_Y;

        // 가로세로비율
        let aspect = width / height;

        // 스크린픽셀 1칸당 월드좌표계 크기
        let x_scale = 2.0 * aspect / (width - 1.0);
        let y_scale = 2.0 / (height - 1.0);

        // 픽셀 좌표를 월드 좌표로 변환 후 월드좌표 범위로 이동
        // Y축은 위쪽이 0이 되도록 반전
        let world_x = position_screen.x * x_scale - aspect;
        let world_y = -(position_screen.y * y_scale - 1.0);

        Vec2::new(world_x, world_y)
    }
}

// eframe::App 트레이트
impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        // 컨트롤러 UI
        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("Circle Controls (World Coords)");

            let aspect =
                consts::MAX_RESOLUTION_X / consts::MAX_RESOLUTION_Y;

            ui.label("Center X:");
            ui.add(
                Slider::new(&mut self.center.x, -aspect..=aspect)
                    .text("X"),
            );

            ui.label("Center Y:");
            ui.add(
                Slider::new(&mut self.center.y, -1.0..=1.0).text("Y"),
            );

            ui.label("Circle Radius:");
            ui.add(
                Slider::new(
                    &mut self.radius,
                    0.0..=consts::MAX_RADIUS,
                )
                .text("Radius"),
            );

            ui.label("Color:");
            ui.color_edit_button_srgba(&mut self.color);
        });

        // 원 그리는 캔버스
        egui::CentralPanel::default().show(ctx, |ui| {
            let width = consts::MAX_RESOLUTION_X as usize;
            let height = consts::MAX_RESOLUTION_Y as usize;

            // 1. 캔버스를 배경색으로 초기화
            let mut image = egui::ColorImage::filled(
                [width, height],
                consts::DEFAULT_BACKGROUND_COLOR,
            );

            // 원의 방정식을 위한 반지름 제곱 (월드 좌표계 기준)
            let radius_squared = self.radius * self.radius;

            // 2. 모든 픽셀을 돌면서 색칠
            for j in 0..height {
                for i in 0..width {
                    let position_screen =
                        Pos2::new(i as f32, j as f32);

                    let position_world = self
                        .transform_screen_to_world(position_screen);

                    // 두 벡터의 차이 계산
                    let distance_squared =
                        (position_world - self.center).length_sq();

                    if distance_squared <= radius_squared {
                        image.pixels[i + width * j] = self.color;
                    }
                }
            }

            // 3. CPU에서 그린 픽셀 데이터를 GPU 텍스처로 로드
            let texture = ctx.load_texture(
                "circle_canvas",
                image,
                egui::TextureOptions::NEAREST,
            );

            // 4. 텍스처를 화면에 그리기
            ui.image((texture.id(), texture.size_vec2()));
        });
    }
}
