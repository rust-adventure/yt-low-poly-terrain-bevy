use bevy::{
    color::palettes::tailwind::*,
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    render::{
        mesh::VertexAttributeValues,
        render_asset::RenderAssetUsages,
        render_resource::{
            Extent3d, TextureDimension, TextureFormat,
        },
        settings::{
            RenderCreation, WgpuFeatures, WgpuSettings,
        },
        RenderPlugin,
    },
};
use bevy_panorbit_camera::{
    PanOrbitCamera, PanOrbitCameraPlugin,
};
use noise::{BasicMulti, NoiseFn, Perlin, Seedable};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(
                    WgpuSettings {
                        // WARN this is a native only feature. It will not work with webgl or webgpu
                        features:
                            WgpuFeatures::POLYGON_MODE_LINE,
                        ..default()
                    },
                ),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin,
            PanOrbitCameraPlugin,
        ))
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            (toggle_wireframe, sync_camera_to_ship),
        )
        .add_systems(Update, control_ship)
        .run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(
            images.add(uv_debug_texture()),
        ),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 20., 75.0)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
            ..default()
        },
        PanOrbitCamera::default(),
        ShipCam,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::default()),
            material: debug_material.clone(),
            transform: Transform::from_xyz(0., 10., 0.),
            ..default()
        },
        Ship,
    ));

    let terrain_height = 70.;
    let noise = BasicMulti::<Perlin>::new(900);

    let mut terrain = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(1000.0, 1000.0)
            .subdivisions(200),
    );

    if let Some(VertexAttributeValues::Float32x3(
        positions,
    )) = terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for pos in positions.iter_mut() {
            let val = noise.get([
                pos[0] as f64 / 300.,
                pos[2] as f64 / 300.,
            ]);

            pos[1] = val as f32 * terrain_height;
        }

        let colors: Vec<[f32; 4]> = positions
            .iter()
            .map(|[_, g, _]| {
                let g = *g / terrain_height * 2.;

                if g > 0.8 {
                    (Color::LinearRgba(LinearRgba {
                        red: 20.,
                        green: 20.,
                        blue: 20.,
                        alpha: 1.,
                    }))
                    .to_linear()
                    .to_f32_array()
                } else if g > 0.3 {
                    Color::from(AMBER_800)
                        .to_linear()
                        .to_f32_array()
                } else if g < -0.8 {
                    Color::BLACK.to_linear().to_f32_array()
                } else {
                    (Color::from(GREEN_400).to_linear())
                        .to_f32_array()
                }
            })
            .collect();
        terrain.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            colors,
        );
    }
    terrain.compute_normals();
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(terrain),
            material: materials.add(Color::WHITE),
            ..default()
        },
        Terrain,
    ));

    // directional 'sun' light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        ..default()
    });
}

#[derive(Component)]
struct Terrain;

fn toggle_wireframe(
    mut commands: Commands,
    landscapes_wireframes: Query<
        Entity,
        (With<Terrain>, With<Wireframe>),
    >,
    landscapes: Query<
        Entity,
        (With<Terrain>, Without<Wireframe>),
    >,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for terrain in &landscapes {
            commands.entity(terrain).insert(Wireframe);
        }
        for terrain in &landscapes_wireframes {
            commands.entity(terrain).remove::<Wireframe>();
        }
    }
}

#[derive(Component)]
struct Ship;

#[derive(Component)]
struct ShipCam;

fn control_ship(
    input: Res<ButtonInput<KeyCode>>,
    mut ships: Query<&mut Transform, With<Ship>>,
) {
    let mut direction = Vec2::new(0., 0.);
    if input.pressed(KeyCode::KeyW) {
        direction.y += 1.;
    }
    if input.pressed(KeyCode::KeyS) {
        direction.y -= 1.;
    }
    if input.pressed(KeyCode::KeyA) {
        direction.x += 1.;
    }
    if input.pressed(KeyCode::KeyD) {
        direction.x -= 1.;
    }
    for mut ship in &mut ships {
        ship.translation.x += direction.x * 1.;
        ship.translation.z += direction.y * 5.;
    }
}

fn sync_camera_to_ship(
    ships: Query<
        &Transform,
        (With<Ship>, Without<ShipCam>),
    >,
    mut camera: Query<&mut PanOrbitCamera, With<ShipCam>>,
) {
    let Ok(ship) = ships.get_single() else {
        return;
    };
    let mut orbit = camera.single_mut();

    orbit.target_focus = Vec3::new(
        ship.translation.x,
        ship.translation.y,
        ship.translation.z,
    );
}

fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255,
        102, 255, 121, 255, 102, 255, 102, 255, 198, 255,
        102, 198, 255, 255, 121, 102, 255, 255, 236, 102,
        255, 255,
    ];

    let mut texture_data =
        [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)]
            .copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
