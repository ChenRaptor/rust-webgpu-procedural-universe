use wgpu::{util::DeviceExt, wgt::PollType};
use std::borrow::Cow;
use std::time::Instant;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeInput {
    value: f32,
    multiplier: f32,
}

async fn run() {
    env_logger::init();
    let start = Instant::now();
    // Créer l'instance wgpu
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Demander un adaptateur
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    // Créer le device et la queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
                trace: Default::default(),
            },
        )
        .await
        .unwrap();

    println!("GPU Device: {}", adapter.get_info().name);

    // Données d'entrée - nous allons calculer value * multiplier pour chaque élément
    let input_data = vec![
        ComputeInput { value: 1.0, multiplier: 2.0 },
        ComputeInput { value: 3.0, multiplier: 4.0 },
        ComputeInput { value: 5.0, multiplier: 6.0 },
        ComputeInput { value: 7.0, multiplier: 8.0 },
        ComputeInput { value: 9.0, multiplier: 10.0 },
        ComputeInput { value: 11.0, multiplier: 12.0 },
        ComputeInput { value: 13.0, multiplier: 14.0 },
        ComputeInput { value: 15.0, multiplier: 16.0 },
    ];

    let data_size = input_data.len();
    println!("Processing {} elements on GPU...", data_size);

    // Buffer d'entrée
    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Input Buffer"),
        contents: bytemuck::cast_slice(&input_data),
        usage: wgpu::BufferUsages::STORAGE,
    });

    // Buffer de sortie
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (data_size * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Buffer de staging pour lire les résultats
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: (data_size * std::mem::size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Compute shader
    let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("compute.wgsl"))),
    });

    // Compute pipeline
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: None,
        module: &compute_shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let offset_data = [0u32];
    let offset_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Offset Buffer"),
        contents: bytemuck::cast_slice(&offset_data),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Bind group
    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Compute Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry { binding: 2, resource: offset_buffer.as_entire_binding() },
        ],
    });

    // Encoder pour les commandes
    // let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //     label: Some("Compute Encoder"),
    // });

    // // Pass de compute
    // {
    //     let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    //         label: Some("Compute Pass"),
    //         timestamp_writes: None,
    //     });
    //     compute_pass.set_pipeline(&compute_pipeline);
    //     compute_pass.set_bind_group(0, &bind_group, &[]);
    //     // Dispatch 8 workgroups (un par élément)
    //     compute_pass.dispatch_workgroups(2, 1, 1);
    // }

    // // Copier le résultat vers le staging buffer
    // encoder.copy_buffer_to_buffer(
    //     &output_buffer,
    //     0,
    //     &staging_buffer,
    //     0,
    //     (data_size * std::mem::size_of::<f32>()) as u64,
    // );

    // // Soumettre les commandes
    // queue.submit(Some(encoder.finish()));

    let workgroup_size = 4; // threads par workgroup (doit correspondre au shader)
    let num_workgroups = 2; // workgroups par dispatch
    let threads_per_dispatch = workgroup_size * num_workgroups;

    let mut offset = 0;
    while offset < data_size {
        let remaining = data_size - offset;
        let dispatch_size = remaining.min(threads_per_dispatch);

        // Mettre à jour le buffer offset
        let offset_value = [offset as u32];
        queue.write_buffer(&offset_buffer, 0, bytemuck::cast_slice(&offset_value));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]); // plus besoin de passer offset ici
            compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            (offset * std::mem::size_of::<f32>()) as u64,
            &staging_buffer,
            (offset * std::mem::size_of::<f32>()) as u64,
            (dispatch_size * std::mem::size_of::<f32>()) as u64,
        );

        queue.submit(Some(encoder.finish()));
        offset += dispatch_size;
    }



    // Mapper le staging buffer pour lire les résultats
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = flume::unbounded();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

        // Poll jusqu'à ce que le mapping soit terminé
    loop {
        device.poll(wgpu::PollType::Wait);
        if receiver.try_recv().is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }


    // Lire les données
    let data = buffer_slice.get_mapped_range();
    let result: &[f32] = bytemuck::cast_slice(&data);

    // Afficher les résultats
    println!("\nRésultats du calcul GPU:");
    println!("Input Data -> GPU Result");
    for (i, input) in input_data.iter().enumerate() {
        println!("{}  * {} = {}", input.value, input.multiplier, result[i]);
    }

    // Calculer la somme totale
    let total: f32 = result.iter().sum();
    println!("\nSomme totale: {}", total);

    // Nettoyer
    drop(data);
    staging_buffer.unmap();
    let duration = start.elapsed();
    println!("Durée d'exécution : {:?}", duration);
}

fn main() {
    pollster::block_on(run());
}
