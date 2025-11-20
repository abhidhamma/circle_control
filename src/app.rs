use eframe::egui;
use egui::{
    Color32, PointerButton, TextureOptions, Vec2 as EguiVec2,
};
use glam::{Vec2, Vec3, vec2, vec3};
use image::GenericImageView;
use rayon::prelude::*;
use std::sync::Arc;

/*
 * Object 트레잇: 씬(Scene)에 포함될 수 있는 모든 객체의 공통 인터페이스
 * Send + Sync: 여러 스레드에서 안전하게 공유 가능 (rayon 병렬 처리용)
*/
trait Object: Send + Sync {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
    fn reflection(&self) -> f32;
    fn transparency(&self) -> f32;
    fn amb_texture(&self) -> Option<Arc<Texture>>;
    fn diff_texture(&self) -> Option<Arc<Texture>>;
}

// 광선 충돌 정보를 담는 구조체
// C++: Hit 클래스
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    uv: Vec2, // 텍스처 좌표
    object: Option<Arc<dyn Object>>,
}

// 광선을 나타내는 구조체
// C++: Ray 클래스
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// 구(Sphere)를 나타내는 구조체
// C++: Sphere 클래스
struct Sphere {
    center: Vec3,
    radius: f32,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
    reflection: f32,
    transparency: f32,
}

// Sphere에 대한 Object 트레잇 구현
impl Object for Sphere {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let oc = ray.origin - self.center;
        let a = ray.direction.length_squared();
        let b = 2.0 * ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                object: None,
            };
        }

        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / (2.0 * a);
        let t2 = (-b + sqrt_discriminant) / (2.0 * a);

        // C++ 코드에서는 min, max를 사용했지만, 물체 안에서 시작하는 광선을 고려하여
        // 먼저 t1 (더 가까운 점)이 양수인지 확인하고, 아니면 t2를 확인
        let distance = if t1 >= 0.0 {
            t1
        } else if t2 >= 0.0 {
            t2
        } else {
            -1.0
        };

        if distance < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                object: None,
            };
        }

        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
            uv: Vec2::ZERO, // 구는 텍스처 좌표 없음
            object: None,
        }
    }

    fn ambient(&self) -> Vec3 {
        self.amb
    }
    fn diffuse(&self) -> Vec3 {
        self.diff
    }
    fn specular(&self) -> Vec3 {
        self.spec
    }
    fn alpha(&self) -> f32 {
        self.alpha
    }
    fn reflection(&self) -> f32 {
        self.reflection
    }
    fn transparency(&self) -> f32 {
        self.transparency
    }
    fn amb_texture(&self) -> Option<Arc<Texture>> {
        None
    }
    fn diff_texture(&self) -> Option<Arc<Texture>> {
        None
    }
}

// 점 광원(Point Light)을 나타내는 구조체
// C++: Light 클래스
struct Light {
    pos: Vec3,
}

// 텍스처 데이터를 담는 구조체
// C++: Texture 클래스
struct Texture {
    width: u32,
    height: u32,
    channels: u32,
    image: Vec<u8>,
}

impl Texture {
    fn from_dynamic_image(img: image::DynamicImage) -> Self {
        let (width, height) = img.dimensions();
        let image_data = img.to_rgba8().into_raw();
        Self {
            width,
            height,
            channels: 4,
            image: image_data,
        }
    }

    // C++: GetWrapped 함수
    fn get_wrapped(&self, mut i: i32, mut j: i32) -> Vec3 {
        i %= self.width as i32;
        j %= self.height as i32;
        if i < 0 {
            i += self.width as i32;
        }
        if j < 0 {
            j += self.height as i32;
        }

        let idx = ((j as u32 * self.width + i as u32) * self.channels)
            as usize;
        let r = self.image[idx] as f32 / 255.0;
        let g = self.image[idx + 1] as f32 / 255.0;
        let b = self.image[idx + 2] as f32 / 255.0;
        vec3(r, g, b)
    }

    // C++: InterpolateBilinear 함수
    fn interpolate_bilinear(
        &self,
        dx: f32,
        dy: f32,
        c00: Vec3,
        c10: Vec3,
        c01: Vec3,
        c11: Vec3,
    ) -> Vec3 {
        let a = c00.lerp(c10, dx);
        let b = c01.lerp(c11, dx);
        a.lerp(b, dy)
    }

    // C++: SampleLinear 함수
    fn sample_linear(&self, uv: Vec2) -> Vec3 {
        let xy = uv * vec2(self.width as f32, self.height as f32)
            - vec2(0.5, 0.5);
        let i = xy.x.floor() as i32;
        let j = xy.y.floor() as i32;
        let dx = xy.x - i as f32;
        let dy = xy.y - j as f32;

        let c00 = self.get_wrapped(i, j);
        let c10 = self.get_wrapped(i + 1, j);
        let c01 = self.get_wrapped(i, j + 1);
        let c11 = self.get_wrapped(i + 1, j + 1);

        self.interpolate_bilinear(dx, dy, c00, c10, c01, c11)
    }
}

// 큐브맵의 각 면을 나타내는 enum
enum CubemapFace {
    Right, // PositiveX (px)
    Left,  // NegativeX (nx)
    Up,    // PositiveY (py)
    Down,  // NegativeY (ny)
    Back,  // PositiveZ (pz)
    Front, // NegativeZ (nz)
}

impl CubemapFace {
    // 각 면에 해당하는 파일 이름을 반환
    fn filename(&self) -> &'static str {
        match self {
            CubemapFace::Right => "right.png",
            CubemapFace::Left => "left.png",
            CubemapFace::Up => "up.png",
            CubemapFace::Down => "down.png",
            CubemapFace::Back => "back.png",
            CubemapFace::Front => "front.png",
        }
    }

    // 컴파일 타임에 이미지 데이터를 포함시켜 반환
    fn bytes(&self) -> &'static [u8] {
        match self {
            CubemapFace::Right => {
                include_bytes!("../assets/cubemap/right.png")
            }
            CubemapFace::Left => {
                include_bytes!("../assets/cubemap/left.png")
            }
            CubemapFace::Up => {
                include_bytes!("../assets/cubemap/up.png")
            }
            CubemapFace::Down => {
                include_bytes!("../assets/cubemap/down.png")
            }
            CubemapFace::Back => {
                include_bytes!("../assets/cubemap/back.png")
            }
            CubemapFace::Front => {
                include_bytes!("../assets/cubemap/front.png")
            }
        }
    }
}

// 큐브맵 구조체
struct Cubemap {
    faces: [Arc<Texture>; 6], // [px, nx, py, ny, pz, nz]
}

impl Cubemap {
    // C++: Cubemap::Cubemap(const char* folder)
    fn load() -> Self {
        const FACES: [CubemapFace; 6] = [
            CubemapFace::Right,
            CubemapFace::Left,
            CubemapFace::Up,
            CubemapFace::Down,
            CubemapFace::Back,
            CubemapFace::Front,
        ];

        let faces: [Arc<Texture>; 6] = FACES.map(|face| {
            let image_bytes = face.bytes();
            let img = image::load_from_memory(image_bytes)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to load image {}: {}",
                        face.filename(),
                        e
                    )
                });
            Arc::new(Texture::from_dynamic_image(img))
        });

        Self { faces }
    }

    // C++: Cubemap::Sample(const vec3& d)
    fn sample(&self, dir: Vec3) -> Vec3 {
        let abs_dir = dir.abs();
        let (face_index, uv) = if abs_dir.x >= abs_dir.y
            && abs_dir.x >= abs_dir.z
        {
            // x-face (Right/Left)
            if dir.x > 0.0 {
                (0, vec2(-dir.z / dir.x, -dir.y / dir.x)) // Right (px)
            } else {
                (1, vec2(dir.z / dir.x, dir.y / dir.x)) // Left (nx)
            }
        } else if abs_dir.y >= abs_dir.x && abs_dir.y >= abs_dir.z {
            // y-face (Up/Down)
            if dir.y > 0.0 {
                (2, vec2(dir.x / dir.y, -dir.z / dir.y)) // Up (py)
            } else {
                (3, vec2(dir.x / dir.y, dir.z / dir.y)) // Down (ny)
            }
        } else {
            // z-face (Back/Front)
            if dir.z > 0.0 {
                (4, vec2(dir.x / dir.z, -dir.y / dir.z)) // Back (pz)
            } else {
                (5, vec2(-dir.x / dir.z, dir.y / dir.z)) // Front (nz)
            }
        };

        // [-1, 1] 범위를 [0, 1] 범위로 변환
        let final_uv = (uv + Vec2::ONE) * 0.5;
        self.faces[face_index].sample_linear(final_uv)
    }
}

// 카메라 구조체
struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
    fov: f32, // 수직 시야각 (degrees)
}

impl Camera {
    // 화면 좌표(i, j)와 화면 크기를 기반으로 주 광선(primary ray)을 생성
    fn generate_ray(
        &self,
        i: f32,
        j: f32,
        width: i32,
        height: i32,
    ) -> Ray {
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward);

        let aspect_ratio = width as f32 / height as f32;
        let fov_rad = self.fov.to_radians();
        let sensor_height = 2.0 * (fov_rad / 2.0).tan();
        let sensor_width = aspect_ratio * sensor_height;

        let u = (i / width as f32 - 0.5) * sensor_width;
        let v = -(j / height as f32 - 0.5) * sensor_height;

        let direction = (forward + u * right + v * up).normalize();

        Ray {
            origin: self.eye,
            direction,
        }
    }
}

// 레이 트레이서
// C++: Raytracer 클래스
struct Raytracer {
    width: i32,
    height: i32,
    objects: Vec<Arc<dyn Object>>,
    light: Light,
    cubemap: Cubemap,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        let sphere1 = Arc::new(Sphere {
            center: vec3(0.0, 0.0, 1.5),
            radius: 1.0,
            amb: vec3(0.0, 0.0, 0.0),
            diff: vec3(0.0, 0.0, 0.0),
            spec: vec3(0.2, 0.2, 0.2),
            alpha: 1000.0,
            reflection: 0.9, // 반사율을 높여 주변을 잘 비추도록 설정
            transparency: 0.0,
        });

        let objects: Vec<Arc<dyn Object>> = vec![sphere1];
        let cubemap = Cubemap::load();

        Self {
            width,
            height,
            objects,
            light: Light {
                pos: vec3(0.0, 5.0, -5.0),
            },
            cubemap,
        }
    }

    // C++: FindClosestCollision 함수
    fn find_closest_collision(&self, ray: &Ray) -> Hit {
        let mut closest_d = f32::MAX;
        let mut closest_hit = Hit {
            distance: -1.0,
            point: Vec3::ZERO,
            normal: Vec3::ZERO,
            uv: Vec2::ZERO,
            object: None,
        };

        for object in &self.objects {
            let mut hit = object.check_ray_collision(ray);
            if hit.distance >= 0.0 && hit.distance < closest_d {
                closest_d = hit.distance;
                hit.object = Some(object.clone());
                closest_hit = hit;
            }
        }
        closest_hit
    }

    /*
        광추적
        1.물체에 부딪치는 빛
        2.물체외부에서 반사되는 빛
        3.물체내부로 굴절되는 빛
        세가지를 계산
    */
    fn trace_ray(&self, ray: &Ray, recurse_level: i32) -> Vec3 {
        if recurse_level < 0 {
            return Vec3::ZERO;
        }

        let hit = self.find_closest_collision(ray);

        if let Some(obj) = hit.object {
            // 1.물체에 부딪치는 빛
            let mut color = Vec3::ZERO;
            let dir_to_light =
                (self.light.pos - hit.point).normalize();
            let mut phong_color = Vec3::ZERO;
            let diff = hit.normal.dot(dir_to_light).max(0.0);
            let reflect_dir =
                2.0 * hit.normal * hit.normal.dot(dir_to_light)
                    - dir_to_light;
            let specular = (-ray.direction)
                .dot(reflect_dir)
                .max(0.0)
                .powf(obj.alpha());

            if let Some(tex) = obj.amb_texture() {
                phong_color +=
                    obj.ambient() * tex.sample_linear(hit.uv);
            } else {
                phong_color += obj.ambient();
            }

            if let Some(tex) = obj.diff_texture() {
                phong_color +=
                    diff * obj.diffuse() * tex.sample_linear(hit.uv);
            } else {
                phong_color += diff * obj.diffuse();
            }

            phong_color += obj.specular() * specular;
            color += phong_color
                * (1.0 - obj.reflection() - obj.transparency());

            // 2.물체외부에서 반사되는 빛
            if obj.reflection() > 0.0 {
                let reflected_direction = (ray.direction
                    - 2.0
                        * ray.direction.dot(hit.normal)
                        * hit.normal)
                    .normalize();
                let reflection_ray = Ray {
                    origin: hit.point + reflected_direction * 1e-4,
                    direction: reflected_direction,
                };
                color += self
                    .trace_ray(&reflection_ray, recurse_level - 1)
                    * obj.reflection();
            }

            // 3.물체내부로 굴절되는 빛(굴절)
            if obj.transparency() > 0.0 {
                const IOR: f32 = 1.5; // 굴절률 (유리)
                let (eta, normal) =
                    if ray.direction.dot(hit.normal) < 0.0 {
                        (1.0 / IOR, hit.normal)
                    } else {
                        (IOR, -hit.normal)
                    };

                let cos_theta1 = (-ray.direction).dot(normal);
                let sin2_theta1 = 1.0 - cos_theta1 * cos_theta1;
                let sin2_theta2 = sin2_theta1 * eta * eta;

                if sin2_theta2 < 1.0 {
                    let cos_theta2 = (1.0 - sin2_theta2).sqrt();
                    let refracted_direction = (ray.direction * eta
                        + normal * (eta * cos_theta1 - cos_theta2))
                        .normalize();

                    let refraction_ray = Ray {
                        origin: hit.point
                            + refracted_direction * 1e-4,
                        direction: refracted_direction,
                    };
                    color += self.trace_ray(
                        &refraction_ray,
                        recurse_level - 1,
                    ) * obj.transparency();
                } else {
                    let reflected_direction = (ray.direction
                        - 2.0
                            * ray.direction.dot(hit.normal)
                            * hit.normal)
                        .normalize();
                    let reflection_ray = Ray {
                        origin: hit.point
                            + reflected_direction * 1e-4,
                        direction: reflected_direction,
                    };
                    color += self.trace_ray(
                        &reflection_ray,
                        recurse_level - 1,
                    ) * obj.transparency();
                }
            }
            color
        } else {
            // 광선이 어떤 물체와도 충돌하지 않으면 큐브맵에서 색상을 샘플링
            self.cubemap.sample(ray.direction)
        }
    }

    // C++: Render 함수
    fn render(
        &self,
        pixels: &mut [Color32],
        camera: &Camera,
        recurse_level: i32,
        width: i32,
        height: i32,
    ) {
        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % width as usize;
            let j = idx / width as usize;

            let pixel_ray = camera
                .generate_ray(i as f32, j as f32, width, height);
            let color_vec = self.trace_ray(&pixel_ray, recurse_level);

            *pixel = Color32::from_rgb(
                (color_vec.x.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.y.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.z.clamp(0.0, 1.0) * 255.0) as u8,
            );
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    #[serde(skip)]
    raytracer: Raytracer,
    #[serde(skip)]
    camera: Camera,
    #[serde(skip)]
    camera_radius: f32,
    #[serde(skip)]
    camera_theta: f32, // 수평각
    #[serde(skip)]
    camera_phi: f32, // 수직각
    #[serde(skip)]
    needs_rerender: bool,
    #[serde(skip)]
    is_dragging: bool, // 사용자가 카메라를 드래그하고 있는지 여부
}

impl Default for TemplateApp {
    fn default() -> Self {
        let radius = 4.0;
        let theta = -std::f32::consts::FRAC_PI_2; // -90도
        let phi = std::f32::consts::FRAC_PI_2; // 90도

        let camera = Camera {
            eye: vec3(
                radius * phi.sin() * theta.cos(),
                radius * phi.cos(),
                radius * phi.sin() * theta.sin(),
            ) + vec3(0.0, 0.0, 1.5), // 구의 중심을 보도록 오프셋
            target: vec3(0.0, 0.0, 1.5), // 구의 중심
            up: Vec3::Y,
            fov: 60.0,
        };

        Self {
            raytracer: Raytracer::new(1280, 720),
            camera,
            camera_radius: radius,
            camera_theta: theta,
            camera_phi: phi,
            needs_rerender: true, // 첫 프레임 렌더링 필요
            is_dragging: false,
        }
    }
}

impl TemplateApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }

    // 카메라 위치 업데이트
    fn update_camera(&mut self) {
        // 구의 중심을 기준으로 회전
        let target = self.camera.target;
        self.camera.eye = vec3(
            self.camera_radius
                * self.camera_phi.sin()
                * self.camera_theta.cos(),
            self.camera_radius * self.camera_phi.cos(),
            self.camera_radius
                * self.camera_phi.sin()
                * self.camera_theta.sin(),
        ) + target;
        self.needs_rerender = true;
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        // input 클로저 밖에서 pixels_per_point를 미리 가져옴
        let was_dragging = self.is_dragging;
        let ppp = ctx.pixels_per_point();

        // 마우스 입력 처리
        ctx.input(|i| {
            // 마우스 휠로 줌인/줌아웃
            if i.raw_scroll_delta.y != 0.0 {
                self.camera_radius *=
                    1.0 - i.raw_scroll_delta.y * 0.05;
                self.camera_radius =
                    self.camera_radius.clamp(2.0, 20.0);
                self.update_camera();
            }

            // 마우스 좌클릭 또는 우클릭 드래그로 카메라 회전
            if i.pointer.is_decidedly_dragging()
                && (i.pointer.button_down(PointerButton::Primary)
                    || i.pointer
                        .button_down(PointerButton::Secondary))
            {
                self.is_dragging = true;
                // 네이티브와 웹 환경의 픽셀 단위 차이를 보정
                // ctx.pixels_per_point()를 곱해줘서 논리적 픽셀을 물리적 픽셀에 가깝게 만듬
                let mut delta = i.pointer.delta();

                // 우클릭 드래그일 때만 스케일링 보정 (더 부드러운 움직임을 위해)
                if i.pointer.button_down(PointerButton::Secondary) {
                    delta *= ppp;
                }
                delta *= ppp;
                if delta != EguiVec2::ZERO {
                    self.camera_theta -= delta.x * 0.01;
                    self.camera_phi -= delta.y * 0.01;
                    // 수직 각도를 제한하여 카메라가 거꾸로 뒤집히는 것을 방지
                    self.camera_phi = self
                        .camera_phi
                        .clamp(0.1, std::f32::consts::PI - 0.1);
                    self.update_camera();
                }
            } else {
                self.is_dragging = false;
            }
        });

        // 드래그가 끝나는 시점에 최종 렌더링을 예약
        if was_dragging && !self.is_dragging {
            self.needs_rerender = true;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // 대화형 모드(드래그 중)일 때 렌더링 옵션 조정
            let (width, height, recurse_level) = if self.is_dragging {
                // 속도를 위해 해상도를 1/4로, 재귀 깊이를 1로 줄임
                (
                    self.raytracer.width / 2,
                    self.raytracer.height / 2,
                    1,
                )
            } else {
                // 최종 렌더링은 최고 품질로
                (self.raytracer.width, self.raytracer.height, 5)
            };

            // 카메라가 움직였을 때만 다시 렌더링
            // 드래그 중일 때는 매 프레임 렌더링
            if self.needs_rerender || self.is_dragging {
                let mut pixels: Vec<Color32> =
                    vec![Color32::BLACK; (width * height) as usize];
                self.raytracer.render(
                    &mut pixels,
                    &self.camera,
                    recurse_level,
                    width,
                    height,
                );

                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    bytemuck::cast_slice(&pixels),
                );

                // 렌더링된 이미지를 egui 컨텍스트 메모리에 저장
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp(
                        egui::Id::new("raytrace_texture"),
                        image,
                    )
                });

                self.needs_rerender = false;
                ctx.request_repaint(); // 다음 프레임에 다시 그리도록 요청
            }

            // 메모리에 저장된 이미지를 불러와서 화면에 표시
            if let Some(image) = ctx.memory(|mem| {
                mem.data.get_temp::<egui::ColorImage>(egui::Id::new(
                    "raytrace_texture",
                ))
            }) {
                let texture_handle = ctx.load_texture(
                    "raytrace_canvas",
                    image.clone(),
                    TextureOptions::NEAREST,
                );
                let img_widget = egui::Image::new(&texture_handle)
                    .fit_to_original_size(1.0)
                    .shrink_to_fit();
                ui.centered_and_justified(|ui| {
                    ui.add(img_widget);
                });
            }
        });
    }
}
