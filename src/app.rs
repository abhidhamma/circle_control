use eframe::egui;
use egui::{Color32, TextureOptions};
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
    PositiveX, // px
    NegativeX, // nx
    PositiveY, // py
    NegativeY, // ny
    PositiveZ, // pz
    NegativeZ, // nz
}

impl CubemapFace {
    // 각 면에 해당하는 파일 이름을 반환
    fn filename(&self) -> &'static str {
        match self {
            CubemapFace::PositiveX => "px.png",
            CubemapFace::NegativeX => "nx.png",
            CubemapFace::PositiveY => "py.jpg",
            CubemapFace::NegativeY => "ny.jpg",
            CubemapFace::PositiveZ => "pz.png",
            CubemapFace::NegativeZ => "nz.png",
        }
    }

    // 컴파일 타임에 이미지 데이터를 포함시켜 반환
    fn bytes(&self) -> &'static [u8] {
        match self {
            CubemapFace::PositiveX => {
                include_bytes!("../assets/cubemap/px.png")
            }
            CubemapFace::NegativeX => {
                include_bytes!("../assets/cubemap/nx.png")
            }
            CubemapFace::PositiveY => {
                include_bytes!("../assets/cubemap/py.jpg")
            }
            CubemapFace::NegativeY => {
                include_bytes!("../assets/cubemap/ny.jpg")
            }
            CubemapFace::PositiveZ => {
                include_bytes!("../assets/cubemap/pz.png")
            }
            CubemapFace::NegativeZ => {
                include_bytes!("../assets/cubemap/nz.png")
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
            CubemapFace::PositiveX,
            CubemapFace::NegativeX,
            CubemapFace::PositiveY,
            CubemapFace::NegativeY,
            CubemapFace::PositiveZ,
            CubemapFace::NegativeZ,
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
            // x-face
            if dir.x > 0.0 {
                (0, vec2(-dir.z / dir.x, -dir.y / dir.x)) // px
            } else {
                (1, vec2(dir.z / dir.x, -dir.y / dir.x)) // nx
            }
        } else if abs_dir.y >= abs_dir.x && abs_dir.y >= abs_dir.z {
            // y-face
            if dir.y > 0.0 {
                (2, vec2(dir.x / dir.y, dir.z / dir.y)) // py
            } else {
                (3, vec2(dir.x / dir.y, -dir.z / dir.y)) // ny
            }
        } else {
            // z-face
            if dir.z > 0.0 {
                (4, vec2(dir.x / dir.z, -dir.y / dir.z)) // pz
            } else {
                (5, vec2(-dir.x / dir.z, -dir.y / dir.z)) // nz
            }
        };

        // [-1, 1] 범위를 [0, 1] 범위로 변환
        let final_uv = (uv + Vec2::ONE) * 0.5;
        self.faces[face_index].sample_linear(final_uv)
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

    // C++: TransformScreenToWorld 함수
    fn transform_screen_to_world(&self, pos_screen: Vec2) -> Vec3 {
        let x_scale = 2.0 / self.width as f32;
        let y_scale = 2.0 / self.height as f32;
        let aspect = self.width as f32 / self.height as f32;

        vec3(
            (pos_screen.x * x_scale - 1.0) * aspect,
            -pos_screen.y * y_scale + 1.0,
            0.0,
        )
    }

    // C++: Render 함수
    fn render(&mut self, pixels: &mut [Color32]) {
        let eye_pos = vec3(0.0, 0.0, -1.5);

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));
            let ray_dir = (pos_world - eye_pos).normalize();
            let pixel_ray = Ray {
                origin: pos_world,
                direction: ray_dir,
            };
            let color_vec = self.trace_ray(&pixel_ray, 5);

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
    is_first_frame: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            raytracer: Raytracer::new(1280, 720),
            is_first_frame: true,
        }
    }
}

impl TemplateApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            // C++의 `if (count == 0)` 와 같이 첫 프레임에만 렌더링
            if self.is_first_frame {
                let mut pixels: Vec<Color32> =
                    vec![Color32::BLACK; width * height];
                self.raytracer.render(&mut pixels);

                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width, height],
                    bytemuck::cast_slice(&pixels),
                );

                // 렌더링된 이미지를 egui 컨텍스트 메모리에 저장
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp(
                        egui::Id::new("raytrace_texture"),
                        image,
                    )
                });

                self.is_first_frame = false;
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
